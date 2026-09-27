//! macOS Now Playing (Control Center, lock screen) and media-key commands for the
//! Player. Registering remote-command handlers makes tbis the "now playing" app, so
//! keyboard ⏯ and AirPods taps reach it instead of launching Music.app.

use futures::channel::mpsc::UnboundedSender;

/// Media-key / Control Center request, forwarded to the Player.
#[derive(Clone, Copy, Debug)]
pub enum RemoteCommand {
    Play,
    Pause,
    Toggle,
    SkipForward,
    SkipBackward,
    Next,
    /// Absolute position in seconds (Control Center scrubber).
    Seek(f64),
}

/// What Now Playing shows; `elapsed` + `rate` let macOS extrapolate the position.
#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub title: String,
    pub subtitle: Option<String>,
    pub duration: f64,
    pub elapsed: f64,
    /// 0 while paused, else playback speed.
    pub rate: f64,
}

#[cfg(target_os = "macos")]
pub use mac::NowPlaying;

#[cfg(not(target_os = "macos"))]
pub struct NowPlaying;

#[cfg(not(target_os = "macos"))]
impl NowPlaying {
    pub fn new(_: UnboundedSender<RemoteCommand>, _: f64) -> Self {
        Self
    }
    pub fn update(&self, _: &Info) {}
    pub fn set_artwork(&self, _: &[u8]) {}
    pub fn set_next_enabled(&self, _: bool) {}
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use std::ptr::NonNull;
    use std::time::Instant;

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{AnyThread as _, MainThreadMarker};
    use objc2_app_kit::NSImage;
    use objc2_core_foundation::CGSize;
    use objc2_foundation::{NSData, NSDictionary, NSMutableDictionary, NSNumber, NSString};
    use objc2_media_player::{
        MPChangePlaybackPositionCommandEvent, MPMediaItemArtwork, MPMediaItemPropertyArtist,
        MPMediaItemPropertyArtwork, MPMediaItemPropertyPlaybackDuration, MPMediaItemPropertyTitle,
        MPNowPlayingInfoCenter, MPNowPlayingInfoMediaType,
        MPNowPlayingInfoPropertyElapsedPlaybackTime, MPNowPlayingInfoPropertyMediaType,
        MPNowPlayingInfoPropertyPlaybackRate, MPNowPlayingPlaybackState, MPRemoteCommand,
        MPRemoteCommandCenter, MPRemoteCommandEvent, MPRemoteCommandHandlerStatus,
    };

    use super::{Info, RemoteCommand, UnboundedSender};

    /// Seconds Now Playing's extrapolated position may drift before republishing.
    const DRIFT: f64 = 1.;

    /// Owns the remote-command registrations; dropping it clears Now Playing.
    pub struct NowPlaying {
        targets: Vec<(Retained<MPRemoteCommand>, Retained<AnyObject>)>,
        next: Retained<MPRemoteCommand>,
        /// Last published info and when, to extrapolate its position.
        info: RefCell<Option<(Info, Instant)>>,
        artwork: RefCell<Option<Retained<MPMediaItemArtwork>>>,
    }

    impl NowPlaying {
        /// `skip` is the seconds a skip-forward/back command moves.
        pub fn new(tx: UnboundedSender<RemoteCommand>, skip: f64) -> Self {
            // SAFETY: MediaPlayer calls on the main thread; handlers only forward to `tx`
            unsafe {
                let center = MPRemoteCommandCenter::sharedCommandCenter();
                let mut targets = Vec::new();
                let mut add = |command: Retained<MPRemoteCommand>, remote: RemoteCommand| {
                    let tx = tx.clone();
                    let block = RcBlock::new(move |_: NonNull<MPRemoteCommandEvent>| {
                        let _ = tx.unbounded_send(remote);
                        MPRemoteCommandHandlerStatus::Success
                    });
                    command.setEnabled(true);
                    let target = command.addTargetWithHandler(&block);
                    targets.push((command, target));
                };
                add(center.playCommand(), RemoteCommand::Play);
                add(center.pauseCommand(), RemoteCommand::Pause);
                add(center.togglePlayPauseCommand(), RemoteCommand::Toggle);
                let interval = NSNumber::new_f64(skip);
                let forward = center.skipForwardCommand();
                forward
                    .setPreferredIntervals(&objc2_foundation::NSArray::from_slice(&[&*interval]));
                add(Retained::into_super(forward), RemoteCommand::SkipForward);
                let backward = center.skipBackwardCommand();
                backward
                    .setPreferredIntervals(&objc2_foundation::NSArray::from_slice(&[&*interval]));
                add(Retained::into_super(backward), RemoteCommand::SkipBackward);
                let next = center.nextTrackCommand();
                add(next.clone(), RemoteCommand::Next);
                next.setEnabled(false);

                let position = center.changePlaybackPositionCommand();
                let block = RcBlock::new(move |event: NonNull<MPRemoteCommandEvent>| {
                    let event = event.cast::<MPChangePlaybackPositionCommandEvent>();
                    let _ = tx.unbounded_send(RemoteCommand::Seek(event.as_ref().positionTime()));
                    MPRemoteCommandHandlerStatus::Success
                });
                position.setEnabled(true);
                let target = position.addTargetWithHandler(&block);
                targets.push((Retained::into_super(position), target));

                Self {
                    targets,
                    next,
                    info: RefCell::new(None),
                    artwork: RefCell::new(None),
                }
            }
        }

        /// Republishes only on a real change: macOS extrapolates position from
        /// elapsed + rate, so steady playback needs no updates, only seeks/drift.
        pub fn update(&self, info: &Info) {
            let stale = match &*self.info.borrow() {
                Some((shown, at)) => {
                    let predicted = shown.elapsed + at.elapsed().as_secs_f64() * shown.rate;
                    shown.title != info.title
                        || shown.subtitle != info.subtitle
                        || shown.duration != info.duration
                        || shown.rate != info.rate
                        || (predicted - info.elapsed).abs() > DRIFT
                }
                None => true,
            };
            if stale {
                *self.info.borrow_mut() = Some((info.clone(), Instant::now()));
                self.publish();
            }
        }

        /// Poster/thumb bytes as downloaded; undecodable data is ignored.
        pub fn set_artwork(&self, bytes: &[u8]) {
            let data = NSData::with_bytes(bytes);
            let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
                return;
            };
            let size = image.size();
            let handler = RcBlock::new(move |_: CGSize| NonNull::from(&*image));
            // SAFETY: handler returns the image it keeps alive
            let artwork = unsafe {
                MPMediaItemArtwork::initWithBoundsSize_requestHandler(
                    MPMediaItemArtwork::alloc(),
                    size,
                    &handler,
                )
            };
            *self.artwork.borrow_mut() = Some(artwork);
            self.publish();
        }

        pub fn set_next_enabled(&self, enabled: bool) {
            // SAFETY: plain property setter on the main thread
            unsafe { self.next.setEnabled(enabled) };
        }

        fn publish(&self) {
            let Some((info, _)) = self.info.borrow().clone() else {
                return;
            };
            let dict: Retained<NSMutableDictionary<NSString, AnyObject>> =
                NSMutableDictionary::new();
            // SAFETY: MediaPlayer's documented keys with their documented value types
            unsafe {
                let set = |key: &NSString, value: &AnyObject| {
                    dict.setObject_forKey(value, objc2::runtime::ProtocolObject::from_ref(key));
                };
                set(MPMediaItemPropertyTitle, &NSString::from_str(&info.title));
                if let Some(subtitle) = &info.subtitle {
                    set(MPMediaItemPropertyArtist, &NSString::from_str(subtitle));
                }
                set(
                    MPMediaItemPropertyPlaybackDuration,
                    &NSNumber::new_f64(info.duration),
                );
                set(
                    MPNowPlayingInfoPropertyElapsedPlaybackTime,
                    &NSNumber::new_f64(info.elapsed),
                );
                set(
                    MPNowPlayingInfoPropertyPlaybackRate,
                    &NSNumber::new_f64(info.rate),
                );
                set(
                    MPNowPlayingInfoPropertyMediaType,
                    &NSNumber::new_usize(MPNowPlayingInfoMediaType::Video.0),
                );
                if let Some(artwork) = self.artwork.borrow().as_ref() {
                    set(MPMediaItemPropertyArtwork, artwork);
                }
                let center = MPNowPlayingInfoCenter::defaultCenter();
                center.setNowPlayingInfo(Some(&dict as &NSDictionary<NSString, AnyObject>));
                center.setPlaybackState(if info.rate > 0. {
                    MPNowPlayingPlaybackState::Playing
                } else {
                    MPNowPlayingPlaybackState::Paused
                });
            }
        }
    }

    impl Drop for NowPlaying {
        fn drop(&mut self) {
            debug_assert!(MainThreadMarker::new().is_some());
            // SAFETY: removing the targets this value registered, on the main thread
            unsafe {
                for (command, target) in &self.targets {
                    command.removeTarget(Some(target));
                }
                self.next.setEnabled(false);
                let center = MPNowPlayingInfoCenter::defaultCenter();
                center.setNowPlayingInfo(None);
                center.setPlaybackState(MPNowPlayingPlaybackState::Stopped);
            }
        }
    }
}
