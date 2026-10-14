use crate::jellyfin::Item;
use crate::tv::Channel;

/// Navigation raised by any screen. `AppView::on_nav` is the single place that
/// turns one of these into a screen change; every emitter just picks an event.
pub enum Nav {
    /// Poster picked; opens that Item's detail page. `Item::kind` says which (Movie
    /// or Series -- an Episode or Other here would be a server data bug).
    Open(Item),
    /// Episode or Movie chosen to play now.
    Play(Item),
    /// Watch tbis TV: the Player joins what the Channel airs now.
    Tune(Channel),
    /// Leave the current detail page.
    Back,
}
