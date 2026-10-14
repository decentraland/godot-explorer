/// Tag naming the resource a worker thread is loading; the engine sets and clears it.
pub const LOADING_RESOURCE_TAG: &str = "loading_resource";

const BYTES_PER_MB: i64 = 1_048_576;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ReportTag {
    pub key: String,
    pub value: String,
}

/// What the shell attaches to every event: tags plus the `graphics` context.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ReportTags {
    pub tags: Vec<ReportTag>,
    pub graphics: Vec<ReportTag>,
}

impl ReportTags {
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|t| t.key == key)
            .map(|t| t.value.as_str())
    }

    fn push(&mut self, key: &str, value: impl Into<String>) {
        self.tags.push(ReportTag {
            key: key.to_string(),
            value: value.into(),
        });
    }

    /// Tags set once at start-up. Empty commit, branch, message and an unknown RAM are omitted.
    pub fn from_context(ctx: &ReportContext, attach_log_sampled: bool) -> Self {
        let mut out = Self::default();
        out.push("attach_log_sampled", attach_log_sampled.to_string());
        out.push("platform", ctx.platform.as_str());
        out.push(
            "build_type",
            if ctx.debug_build { "debug" } else { "release" },
        );
        out.push("gpu_vendor", ctx.gpu_vendor.as_str());
        out.push("locale", ctx.locale.as_str());
        if !ctx.commit_hash.is_empty() {
            out.push("commit_hash", ctx.commit_hash.as_str());
        }
        if let Some(bytes) = ctx.physical_ram_bytes.filter(|b| *b > 0) {
            out.push("total_ram_mb", (bytes / BYTES_PER_MB).to_string());
        }
        if ctx.attach_branch_info {
            if !ctx.branch_name.is_empty() {
                out.push("branch_name", ctx.branch_name.as_str());
            }
            if !ctx.commit_message.is_empty() {
                out.push("commit_message", ctx.commit_message.as_str());
            }
        }
        for (key, value) in [
            ("name", &ctx.gpu_name),
            ("vendor", &ctx.gpu_vendor),
            ("version", &ctx.gpu_api_version),
        ] {
            out.graphics.push(ReportTag {
                key: key.to_string(),
                value: value.clone(),
            });
        }
        out
    }
}

/// Facts about the build and device that the tags come from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ReportContext {
    pub platform: String,
    pub debug_build: bool,
    pub gpu_name: String,
    pub gpu_vendor: String,
    pub gpu_api_version: String,
    pub locale: String,
    pub commit_hash: String,
    /// `None` where the OS does not report it (Android; the host supplies the tag there).
    pub physical_ram_bytes: Option<i64>,
    /// Staging and development builds only.
    pub attach_branch_info: bool,
    pub branch_name: String,
    pub commit_message: String,
}
