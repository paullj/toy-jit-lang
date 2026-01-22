use std::path::PathBuf;

pub enum CompileProfile {
    Debug,
    Release,
}

pub enum CompileSource {
    Script(PathBuf),
    Project(PathBuf),
}

pub struct CompileOptions {
    pub source: CompileSource,
    pub profile: CompileProfile,
}

impl CompileOptions {
    pub fn new(path: PathBuf) -> Self {
        Self {
            profile: CompileProfile::Debug,
            source: CompileSource::Script(path),
        }
    }

    pub fn with_profile(mut self, profile: CompileProfile) -> Self {
        self.profile = profile;
        self
    }
}
