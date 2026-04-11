// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! CursorWrap Core — pure-logic cursor wrapping engine.
//!
//! This crate contains the monitor topology, edge detection, and wrap logic
//! without any Win32 dependencies. It is designed for comprehensive testing.

pub mod types;
pub mod topology;
pub mod wrap_core;
