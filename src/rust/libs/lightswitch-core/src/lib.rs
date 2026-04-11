// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! LightSwitch core logic — platform-independent, no Win32 dependencies.
//!
//! This crate contains settings parsing, schedule logic (time-based theme
//! switching, sunrise/sunset calculation), and state management for the
//! LightSwitch module. All system-dependent operations (registry, time)
//! are injected, making the logic fully testable.

pub mod settings;
pub mod schedule;
pub mod sun;
pub mod state;
