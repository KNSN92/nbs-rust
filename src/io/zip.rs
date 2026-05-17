#[cfg(feature = "audio")]
use crate::audio::FileAudioProvider;
use crate::{Nbs, io::NbsIOError};

#[cfg(not(feature = "audio"))]
pub fn extract_nbs(zip_bytes: &[u8]) -> Result<Nbs, NbsIOError> {
    todo!()
}

#[cfg(feature = "audio")]
pub fn extract_nbs(zip_bytes: &[u8]) -> Result<(Nbs, FileAudioProvider), NbsIOError> {
    todo!()
}

#[cfg(feature = "audio")]
pub fn bundle_nbs(nbs: &Nbs, audio_provider: FileAudioProvider) -> Result<Vec<u8>, NbsIOError> {
    todo!()
}
