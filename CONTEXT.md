# tbis

A desktop Jellyfin client for browsing a user's Movies and Series and playing them with mpv.

## Library

**Library**:
A Jellyfin media folder on the server. The app always merges every Library, so the user never picks one.
_Avoid_: Folder, collection

**Movie**:
A single playable film.

**Series**:
A show made of Episodes grouped in seasons. It is fully played only when every Episode is played.
_Avoid_: Show, TV show

**Episode**:
A single playable part of a Series.

## Home

**Continue Watching**:
A Home row of movies and episodes the user has started but not finished, with the most recently watched first.
_Avoid_: Resume, in progress

**Next Up**:
A Home row with the next unwatched episode of each Series the user is following. It never contains movies, and it never repeats an item that is already in Continue Watching.
_Avoid_: Next watch, up next

**Movies row**:
A Home row of unstarted Movies from all Libraries, with the most recently added first.

**Series row**:
A Home row of Series that are not fully played, from all Libraries, ordered by when their newest Episode was added.

## Screens

**Search**:
The screen of server-side results for the title-bar query. It covers Movies, Series, and Episodes from all Libraries.
_Avoid_: Filter, find

**Movie detail**:
The screen for one Movie, showing its poster, meta, and overview, with a Play or Resume button.

**Series detail**:
The screen for one Series, where the user picks a season and plays an Episode.

**Player**:
The in-window screen that plays one Movie or Episode, starting from its saved position when there is one.

**Autoplay**:
In the last 30 seconds of an Episode, the Player offers the next Episode of the same Series and plays it when the Episode ends. The user can play it right away or cancel. There is no Autoplay after a Movie or after the last Episode.
_Avoid_: Binge, continuous play

**PiP**:
A small, always-on-top, borderless window that takes over playback from the Player until it is closed.
_Avoid_: Mini player, picture-in-picture window
