use futures::channel::mpsc::UnboundedSender;
use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, PlaybackPositionChangeRequestedEventArgs,
    SystemMediaTransportControls, SystemMediaTransportControlsButton,
    SystemMediaTransportControlsButtonPressedEventArgs,
    SystemMediaTransportControlsTimelineProperties,
};
use windows::Storage::Streams::{IRandomAccessStream, RandomAccessStreamReference};
use windows::Win32::System::WinRT::{
    BSOS_DEFAULT, CreateRandomAccessStreamOverStream, ISystemMediaTransportControlsInterop,
};
use windows::Win32::UI::Shell::SHCreateMemStream;
use windows::core::{HSTRING, factory};

use crate::now_playing::{Info, RemoteCommand};

pub struct System(Option<Session>);

struct Session {
    controls: SystemMediaTransportControls,
    button: Option<i64>,
    position: Option<i64>,
}

fn report(result: windows::core::Result<()>) {
    if let Err(error) = result {
        crate::log::write(&format!("SMTC failed: {:?}", error.code()));
    }
}

impl System {
    pub fn new(window: &gpui_kit::Window, tx: UnboundedSender<RemoteCommand>, _: f64) -> Self {
        let result = super::hwnd(window).and_then(|hwnd| {
            Self::create(hwnd, tx)
                .map_err(|error| format!("SMTC initialization failed: {:?}", error.code()))
        });
        match result {
            Ok(session) => Self(Some(session)),
            Err(error) => {
                crate::log::write(&error);
                Self(None)
            }
        }
    }

    fn create(
        hwnd: windows::Win32::Foundation::HWND,
        tx: UnboundedSender<RemoteCommand>,
    ) -> windows::core::Result<Session> {
        let interop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()?;
        let controls: SystemMediaTransportControls = unsafe { interop.GetForWindow(hwnd)? };
        let mut session = Session {
            controls,
            button: None,
            position: None,
        };
        session.controls.SetIsPlayEnabled(true)?;
        session.controls.SetIsPauseEnabled(true)?;
        session.controls.SetIsFastForwardEnabled(true)?;
        session.controls.SetIsRewindEnabled(true)?;
        let buttons = tx.clone();
        session.button =
            Some(session.controls.ButtonPressed(&TypedEventHandler::<
                SystemMediaTransportControls,
                SystemMediaTransportControlsButtonPressedEventArgs,
            >::new(move |_, args| {
                let command = match args.ok()?.Button()? {
                    SystemMediaTransportControlsButton::Play => Some(RemoteCommand::Play),
                    SystemMediaTransportControlsButton::Pause => Some(RemoteCommand::Pause),
                    SystemMediaTransportControlsButton::Next => Some(RemoteCommand::Next),
                    SystemMediaTransportControlsButton::Previous => Some(RemoteCommand::Previous),
                    SystemMediaTransportControlsButton::FastForward => {
                        Some(RemoteCommand::SkipForward)
                    }
                    SystemMediaTransportControlsButton::Rewind => Some(RemoteCommand::SkipBackward),
                    _ => None,
                };
                if let Some(command) = command {
                    let _ = buttons.unbounded_send(command);
                }
                Ok(())
            }))?);
        session.position = Some(session.controls.PlaybackPositionChangeRequested(
            &TypedEventHandler::<
                SystemMediaTransportControls,
                PlaybackPositionChangeRequestedEventArgs,
            >::new(move |_, args| {
                let seconds = args.ok()?.RequestedPlaybackPosition()?.Duration as f64 / 10_000_000.;
                let _ = tx.unbounded_send(RemoteCommand::Seek(seconds));
                Ok(())
            }),
        )?);
        session.controls.SetIsEnabled(true)?;
        Ok(session)
    }

    pub fn update(&self, info: &Info) {
        let Some(session) = &self.0 else { return };
        report((|| {
            let controls = &session.controls;
            let updater = controls.DisplayUpdater()?;
            updater.SetType(MediaPlaybackType::Video)?;
            let video = updater.VideoProperties()?;
            video.SetTitle(&HSTRING::from(&info.title))?;
            video.SetSubtitle(&HSTRING::from(info.subtitle.as_deref().unwrap_or_default()))?;
            updater.Update()?;
            controls.SetPlaybackStatus(if info.rate > 0. {
                MediaPlaybackStatus::Playing
            } else {
                MediaPlaybackStatus::Paused
            })?;
            if info.rate > 0. {
                controls.SetPlaybackRate(info.rate)?;
            }
            let span = |seconds: f64| TimeSpan {
                Duration: (seconds.max(0.) * 10_000_000.) as i64,
            };
            let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
            timeline.SetStartTime(span(0.))?;
            timeline.SetEndTime(span(info.duration))?;
            timeline.SetMinSeekTime(span(0.))?;
            timeline.SetMaxSeekTime(span(info.duration))?;
            timeline.SetPosition(span(info.elapsed.clamp(0., info.duration.max(0.))))?;
            controls.UpdateTimelineProperties(&timeline)
        })());
    }

    pub fn set_artwork(&self, bytes: &[u8]) {
        let Some(session) = &self.0 else { return };
        if u32::try_from(bytes.len()).is_err() {
            return;
        }
        report((|| {
            let stream = unsafe { SHCreateMemStream(Some(bytes)) }.ok_or_else(|| {
                windows::core::Error::from_hresult(windows::Win32::Foundation::E_OUTOFMEMORY)
            })?;
            let stream: IRandomAccessStream =
                unsafe { CreateRandomAccessStreamOverStream(&stream, BSOS_DEFAULT)? };
            let reference = RandomAccessStreamReference::CreateFromStream(&stream)?;
            let updater = session.controls.DisplayUpdater()?;
            updater.SetThumbnail(&reference)?;
            updater.Update()
        })());
    }

    pub fn set_episode_nav(&self, previous: bool, next: bool) {
        let Some(session) = &self.0 else { return };
        report(session.controls.SetIsPreviousEnabled(previous));
        report(session.controls.SetIsNextEnabled(next));
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(token) = self.button.take() {
            report(self.controls.RemoveButtonPressed(token));
        }
        if let Some(token) = self.position.take() {
            report(self.controls.RemovePlaybackPositionChangeRequested(token));
        }
        report(self.controls.SetIsEnabled(false));
        report(
            self.controls
                .SetPlaybackStatus(MediaPlaybackStatus::Stopped),
        );
        report((|| {
            let updater = self.controls.DisplayUpdater()?;
            updater.ClearAll()?;
            updater.Update()
        })());
    }
}
