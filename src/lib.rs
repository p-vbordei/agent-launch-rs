//! agent-launch — draft platform-native release announcements.
//!
//! Rust port of [@p-vbordei/agent-launch](https://github.com/p-vbordei/agent-launch).

pub mod config;
pub mod context;
pub mod draft;
pub mod platforms;

pub use config::{
    load_launch_config, Context as LaunchContext, LaunchConfig, LaunchConfigError, Platform,
    Project,
};
pub use context::{gather_context, ContextError, GatheredContext};
pub use draft::{draft_one, AnthropicClient, DraftResult};
pub use platforms::{list_platforms, load_platform_template, PlatformKind, PlatformTemplate};
