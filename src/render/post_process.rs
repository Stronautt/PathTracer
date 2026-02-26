// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostEffect {
    None,
    Negative,
    Sepia,
    Grayscale,
    Fxaa,
    OilPainting,
    BlackAndWhite,
    Comic,
    Casting,
}

impl PostEffect {
    pub fn as_u32(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Negative => 1,
            Self::Sepia => 2,
            Self::Grayscale => 3,
            Self::Fxaa => 4,
            Self::OilPainting => 5,
            Self::BlackAndWhite => 6,
            Self::Comic => 7,
            Self::Casting => 8,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Negative => "Negative",
            Self::Sepia => "Sepia",
            Self::Grayscale => "Grayscale",
            Self::Fxaa => "FXAA",
            Self::OilPainting => "Oil Painting",
            Self::BlackAndWhite => "B&W",
            Self::Comic => "Comic",
            Self::Casting => "Casting",
        }
    }

    pub const ALL: &[Self] = &[
        Self::None,
        Self::Negative,
        Self::Sepia,
        Self::Grayscale,
        Self::Fxaa,
        Self::OilPainting,
        Self::BlackAndWhite,
        Self::Comic,
        Self::Casting,
    ];

    /// All effects except None (for multi-select UI).
    pub const ALL_EFFECTS: &[Self] = &[
        Self::Negative,
        Self::Sepia,
        Self::Grayscale,
        Self::Fxaa,
        Self::OilPainting,
        Self::BlackAndWhite,
        Self::Comic,
        Self::Casting,
    ];
}
