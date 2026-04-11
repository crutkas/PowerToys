// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! FindMyMouse core logic — platform-independent, no Win32 dependencies.
//!
//! This crate contains the detection state machines and settings for the
//! FindMyMouse module. All time values are injected (no system calls),
//! making the logic fully testable.

pub mod types;
pub mod detector;
pub mod shake_detector;
pub mod settings;
