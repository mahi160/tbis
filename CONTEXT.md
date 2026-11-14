# tbis

A desktop Jellyfin client for browsing a user's Movies and Series and playing them with mpv.

## Library

**Library**:
A Jellyfin media folder on the server. The Default Home layout merges every Library; the Modern layout shows one row per Movie or Series Library. With the Libraries setting on Separate, Movies and Series get a tab per Library (after All) to narrow the grid to one; the pick lasts until the app quits.
_Avoid_: Folder, collection

**Movie**:
A single playable film.

**Series**:
A show made of Episodes grouped in seasons. It is fully played only when every Episode is played.
_Avoid_: Show, TV show

**Episode**:
A single playable part of a Series.

## Home

**Hero**:
The carousel at the top of the Modern Home layout: the five newest Movies the user hasn't started, from all Libraries. Each slide shows the Movie's logo or title, meta, overview, and Play and More info buttons. The previous and next films peek in at the edges. It advances on its own and pauses while the pointer is on it; dots below or a peek pick a slide.
_Avoid_: Banner, spotlight

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
A small, always-on-top window that shows the Player's video, with play/pause and a way back to the Player. Playback carries on uninterrupted when entering or leaving it.
_Avoid_: Mini player, picture-in-picture window
