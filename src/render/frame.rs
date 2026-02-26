// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::constants::WORKGROUP_SIZE;
use crate::gpu::buffers::dispatch_size;

pub fn dispatch_path_trace(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::ComputePipeline,
    bind_groups: &[&wgpu::BindGroup],
    width: u32,
    height: u32,
) {
    dispatch_compute(
        encoder,
        pipeline,
        bind_groups,
        width,
        height,
        "path trace pass",
    );
}

pub fn dispatch_post_process(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    width: u32,
    height: u32,
) {
    dispatch_compute(
        encoder,
        pipeline,
        &[bind_group],
        width,
        height,
        "post process pass",
    );
}

fn dispatch_compute(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::ComputePipeline,
    bind_groups: &[&wgpu::BindGroup],
    width: u32,
    height: u32,
    label: &str,
) {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some(label),
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    for (i, bg) in bind_groups.iter().enumerate() {
        pass.set_bind_group(i as u32, Some(*bg), &[]);
    }
    pass.dispatch_workgroups(
        dispatch_size(width, WORKGROUP_SIZE),
        dispatch_size(height, WORKGROUP_SIZE),
        1,
    );
}
