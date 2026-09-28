use crate::config::Config;
use crate::lean::runner::{VerifyOutcome, verify_once};
use crate::lean::sandbox::SandboxExecutor;

pub trait LeanExecutor: Send + Sync {
    fn verify<'a>(
        &'a self,
        config: &'a Config,
        source: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = VerifyOutcome> + Send + 'a>>;
}

pub struct LocalExecutor;

impl LeanExecutor for LocalExecutor {
    fn verify<'a>(
        &'a self,
        config: &'a Config,
        source: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = VerifyOutcome> + Send + 'a>> {
        Box::pin(async move {
            if config.sandbox_enabled {
                panic!("SECURITY VIOLATION: LocalExecutor invoked when sandbox_enabled=true!");
            }
            verify_once(config, source).await
        })
    }
}

pub fn get_executor(config: &Config) -> Box<dyn LeanExecutor> {
    if config.sandbox_enabled {
        Box::new(SandboxExecutor)
    } else {
        Box::new(LocalExecutor)
    }
}
