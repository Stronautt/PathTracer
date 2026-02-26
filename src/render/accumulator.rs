// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Instant;

pub struct Accumulator {
    pub sample_count: u32,
    pub render_start: Instant,
    dirty: bool,
}

impl Default for Accumulator {
    fn default() -> Self {
        Self {
            sample_count: 0,
            dirty: true,
            render_start: Instant::now(),
        }
    }
}

impl Accumulator {
    /// Mark that the scene/camera changed and accumulation must restart.
    pub fn reset(&mut self) {
        self.sample_count = 0;
        self.dirty = true;
        self.render_start = Instant::now();
    }

    /// Advance to the next sample. Returns true if the accumulation buffer needs clearing.
    pub fn advance(&mut self) -> bool {
        self.sample_count += 1;
        let needs_clear = self.dirty;
        self.dirty = false;
        needs_clear
    }

    pub fn needs_reset(&self) -> bool {
        self.dirty
    }
}
