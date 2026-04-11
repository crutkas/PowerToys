// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Mouse Highlighter core logic — platform-independent, no Win32 dependencies.
//!
//! This crate models the highlight lifecycle: creation on mouse-down, position
//! tracking while held, fade-out on mouse-up, and automatic cleanup once fully
//! transparent. An "always" highlight follows the cursor when no button is held.

pub mod highlight_manager;
pub mod settings;
pub mod types;
