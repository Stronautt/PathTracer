// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use bytemuck::{Pod, Zeroable};

use super::aabb::Aabb;
use crate::constants::{BVH_LEAF_MAX_PRIMS, BVH_NUM_BINS};

/// GPU BVH node. The left child is always stored at `index + 1` in the flat
/// array; `left_or_prim` holds the right child index for inner nodes and the
/// first primitive index for leaf nodes. `prim_count == 0` means inner node.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuBvhNode {
    pub aabb_min: [f32; 3],
    pub left_or_prim: u32,
    pub aabb_max: [f32; 3],
    pub prim_count: u32,
}

struct BvhBuildNode {
    bounds: Aabb,
    left: Option<usize>,
    right: Option<usize>,
    first_prim: usize,
    prim_count: usize,
}

/// Flat BVH built over a primitive AABB list, ready for GPU upload.
pub struct Bvh {
    pub nodes: Vec<GpuBvhNode>,
    pub prim_indices: Vec<u32>,
}

impl Bvh {
    /// Build a BVH over `aabbs` using the Surface Area Heuristic.
    pub fn build(aabbs: &[Aabb]) -> Self {
        if aabbs.is_empty() {
            return Self {
                nodes: vec![GpuBvhNode::zeroed()],
                prim_indices: vec![],
            };
        }

        let mut indices: Vec<usize> = (0..aabbs.len()).collect();
        let mut build_nodes: Vec<BvhBuildNode> = Vec::with_capacity(2 * aabbs.len());
        Self::build_recursive(aabbs, &mut indices, 0, aabbs.len(), &mut build_nodes);

        let mut nodes = Vec::with_capacity(build_nodes.len());
        Self::flatten(&build_nodes, 0, &mut nodes);

        let prim_indices = indices.iter().map(|&i| i as u32).collect();
        Self {
            nodes,
            prim_indices,
        }
    }

    fn build_recursive(
        aabbs: &[Aabb],
        indices: &mut [usize],
        start: usize,
        end: usize,
        nodes: &mut Vec<BvhBuildNode>,
    ) -> usize {
        let count = end - start;
        let bounds = indices[start..end]
            .iter()
            .fold(Aabb::EMPTY, |acc, &i| acc.union(aabbs[i]));
        let node_idx = nodes.len();

        if count <= BVH_LEAF_MAX_PRIMS {
            nodes.push(BvhBuildNode {
                bounds,
                left: None,
                right: None,
                first_prim: start,
                prim_count: count,
            });
            return node_idx;
        }

        let (best_axis, best_split) = Self::find_best_split(aabbs, &indices[start..end], &bounds);
        let raw_mid =
            Self::partition(aabbs, &mut indices[start..end], best_axis, best_split) + start;

        // If SAH produced a degenerate partition, fall back to a median split.
        let mid = if raw_mid == start || raw_mid == end {
            (start + end) / 2
        } else {
            raw_mid
        };

        // Push a placeholder; children fill in `left`/`right` after recursion.
        nodes.push(BvhBuildNode {
            bounds,
            left: None,
            right: None,
            first_prim: 0,
            prim_count: 0,
        });

        let left = Self::build_recursive(aabbs, indices, start, mid, nodes);
        let right = Self::build_recursive(aabbs, indices, mid, end, nodes);
        nodes[node_idx].left = Some(left);
        nodes[node_idx].right = Some(right);

        node_idx
    }

    fn find_best_split(aabbs: &[Aabb], indices: &[usize], parent_bounds: &Aabb) -> (usize, f32) {
        let mut best_cost = f32::INFINITY;
        let mut best_axis = 0;
        let mut best_split = 0.0f32;

        for axis in 0..3 {
            let min = parent_bounds.min[axis];
            let max = parent_bounds.max[axis];
            let extent = max - min;
            if extent.abs() < 1e-8 {
                continue;
            }

            // Phase 1: Bin all primitives by centroid — O(N) per axis.
            let mut bin_bounds = [Aabb::EMPTY; BVH_NUM_BINS];
            let mut bin_counts = [0u32; BVH_NUM_BINS];
            let inv_extent = BVH_NUM_BINS as f32 / extent;
            for &idx in indices {
                let centroid = aabbs[idx].center()[axis];
                let b = ((centroid - min) * inv_extent) as usize;
                let b = b.min(BVH_NUM_BINS - 1);
                bin_bounds[b] = bin_bounds[b].union(aabbs[idx]);
                bin_counts[b] += 1;
            }

            // Phase 2: Right-to-left sweep — accumulate right-side bounds/counts.
            let mut right_area = [0.0f32; BVH_NUM_BINS - 1];
            let mut right_count = [0u32; BVH_NUM_BINS - 1];
            {
                let mut rb = Aabb::EMPTY;
                let mut rc = 0u32;
                for i in (1..BVH_NUM_BINS).rev() {
                    rb = rb.union(bin_bounds[i]);
                    rc += bin_counts[i];
                    right_area[i - 1] = rb.surface_area();
                    right_count[i - 1] = rc;
                }
            }

            // Phase 3: Left-to-right sweep — evaluate SAH cost at each split.
            let mut lb = Aabb::EMPTY;
            let mut lc = 0u32;
            let bin_width = extent / BVH_NUM_BINS as f32;
            for i in 0..(BVH_NUM_BINS - 1) {
                lb = lb.union(bin_bounds[i]);
                lc += bin_counts[i];
                if lc == 0 || right_count[i] == 0 {
                    continue;
                }

                let cost = lc as f32 * lb.surface_area() + right_count[i] as f32 * right_area[i];

                if cost < best_cost {
                    best_cost = cost;
                    best_axis = axis;
                    best_split = min + (i + 1) as f32 * bin_width;
                }
            }
        }

        (best_axis, best_split)
    }

    fn partition(aabbs: &[Aabb], indices: &mut [usize], axis: usize, split: f32) -> usize {
        let mut lo = 0;
        let mut hi = indices.len();
        while lo < hi {
            if aabbs[indices[lo]].center()[axis] < split {
                lo += 1;
            } else {
                hi -= 1;
                indices.swap(lo, hi);
            }
        }
        lo
    }

    fn flatten(build_nodes: &[BvhBuildNode], idx: usize, output: &mut Vec<GpuBvhNode>) {
        let node = &build_nodes[idx];
        let out_idx = output.len();

        if node.prim_count > 0 {
            output.push(GpuBvhNode {
                aabb_min: node.bounds.min.into(),
                left_or_prim: node.first_prim as u32,
                aabb_max: node.bounds.max.into(),
                prim_count: node.prim_count as u32,
            });
        } else {
            // Left child immediately follows this node; right child index is
            // patched in after the left subtree is fully written.
            output.push(GpuBvhNode {
                aabb_min: node.bounds.min.into(),
                left_or_prim: 0,
                aabb_max: node.bounds.max.into(),
                prim_count: 0,
            });
            Self::flatten(build_nodes, node.left.unwrap(), output);
            let right_idx = output.len() as u32;
            output[out_idx].left_or_prim = right_idx;
            Self::flatten(build_nodes, node.right.unwrap(), output);
        }
    }
}
