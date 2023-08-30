use deno_core::{error::AnyError, OpState};
use deno_fetch::FetchPermissions;
use deno_web::TimersPermission;

// TODO: fetch

// we have to provide fetch perm structs even though we don't use them
pub struct FP;
impl FetchPermissions for FP {
    fn check_net_url(&mut self, _: &deno_core::url::Url, _: &str) -> Result<(), AnyError> {
        panic!();
    }

    fn check_read(&mut self, _: &std::path::Path, _: &str) -> Result<(), AnyError> {
        panic!();
    }
}

pub struct TP;
impl TimersPermission for TP {
    fn allow_hrtime(&mut self) -> bool {
        false
    }

    fn check_unstable(&self, _: &OpState, _: &'static str) {
        panic!("i don't know what this is for")
    }
}
