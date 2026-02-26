use anyhow::Result;

mod backend_service;

pub fn run_backend() -> Result<()> {
    backend_service::run()
}
