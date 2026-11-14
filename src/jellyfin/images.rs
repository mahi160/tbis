use anyhow::Result;
use futures::AsyncReadExt as _;
use gpui_kit::http_client::{AsyncBody, Request};

use super::Api;
use super::types::{Item, Kind, Person, PlaybackItem, TrickplayInfo};

impl Api {
    /// One sprite-sheet tile (a grid of `info.tile_width`x`info.tile_height` thumbnails).
    pub fn trickplay_tile_url(
        &self,
        item: &PlaybackItem,
        info: &TrickplayInfo,
        tile: i32,
    ) -> String {
        format!(
            // img() can't send headers; trickplay (unlike Images) requires auth
            "{}/Videos/{}/Trickplay/{}/{tile}.jpg?mediaSourceId={}&api_key={}",
            self.server,
            item.id,
            info.width,
            item.media_source_id(),
            self.session.token
        )
    }

    /// 16:9 art. Episode: still, Series thumb, Series backdrop. Movie: thumb, backdrop, poster.
    pub fn wide_image_url(&self, item: &Item) -> Option<String> {
        let own = |kind: &'static str, tag: Option<&String>| {
            tag.map(|t| (item.id.clone(), kind, t.clone()))
        };
        let parent = |kind: &'static str, id: &Option<String>, tag: Option<&String>| {
            id.clone().zip(tag.cloned()).map(|(id, t)| (id, kind, t))
        };
        let pick = if item.kind == Kind::Episode {
            own("Primary", item.image_tags.get("Primary"))
                .or_else(|| {
                    parent(
                        "Thumb",
                        &item.parent_thumb_item_id,
                        item.parent_thumb_image_tag.as_ref(),
                    )
                })
                .or_else(|| {
                    parent(
                        "Backdrop/0",
                        &item.parent_backdrop_item_id,
                        item.parent_backdrop_image_tags.first(),
                    )
                })
        } else {
            own("Thumb", item.image_tags.get("Thumb"))
                .or_else(|| own("Backdrop/0", item.backdrop_image_tags.first()))
                .or_else(|| own("Primary", item.image_tags.get("Primary")))
        };
        let (id, kind, tag) = pick?;
        // wide_w is 288 pt; 600 px covers a 2x display
        Some(format!(
            "{}/Items/{id}/Images/{kind}?fillWidth=600&quality=90&tag={tag}",
            self.server
        ))
    }

    /// Raw bytes of an image URL from this API (e.g. Now Playing artwork).
    pub async fn image_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let request = Request::builder()
            .uri(url)
            .header("Authorization", self.auth_header())
            .body(AsyncBody::empty())?;
        let mut bytes = Vec::new();
        self.send(request)
            .await?
            .into_body()
            .read_to_end(&mut bytes)
            .await?;
        Ok(bytes)
    }

    /// Full-width backdrop art for detail pages.
    /// Own backdrop, else (an Episode's) its Series' one.
    pub fn backdrop_url(&self, item: &Item) -> Option<String> {
        let (id, tag) = match item.backdrop_image_tags.first() {
            Some(tag) => (&item.id, tag),
            None => (
                item.parent_backdrop_item_id.as_ref()?,
                item.parent_backdrop_image_tags.first()?,
            ),
        };
        Some(format!(
            "{}/Items/{id}/Images/Backdrop/0?fillWidth=1920&quality=85&tag={tag}",
            self.server
        ))
    }

    /// Title logo (transparent title art) for the hero.
    pub fn logo_url(&self, item: &Item) -> Option<String> {
        let tag = item.image_tags.get("Logo")?;
        Some(format!(
            "{}/Items/{}/Images/Logo?maxWidth=800&quality=90&tag={tag}",
            self.server, item.id
        ))
    }

    pub fn person_image_url(&self, person: &Person) -> Option<String> {
        let tag = person.primary_image_tag.as_ref()?;
        Some(format!(
            "{}/Items/{}/Images/Primary?fillWidth=128&quality=90&tag={tag}",
            self.server, person.id
        ))
    }

    pub fn poster_url(&self, item: &Item) -> Option<String> {
        let tag = item.image_tags.get("Primary")?;
        Some(format!(
            "{}/Items/{}/Images/Primary?fillWidth=320&quality=90&tag={tag}",
            self.server, item.id
        ))
    }
}

/// `url` (an image URL from this API) blurred server-side, for spoilers; gpui can't blur.
pub fn blurred(url: &str) -> String {
    format!("{url}&blur=30")
}

#[cfg(test)]
mod tests {
    use super::blurred;
    use crate::jellyfin::{Api, Item, Session};
    use std::sync::Arc;

    #[test]
    fn logo_url_needs_a_logo_tag() {
        let (tx, _rx) = futures::channel::mpsc::unbounded();
        let session = Session {
            server: "http://s".into(),
            user_id: "u1".into(),
            user_name: "mahi".into(),
            token: "tok".into(),
        };
        let http = Arc::new(reqwest_client::ReqwestClient::new());
        let api = Api::new(http, session, "http://s".into(), "dev".into(), tx);
        let item = |tags| -> Item {
            serde_json::from_value(
                serde_json::json!({"Id": "i", "Name": "Movie", "ImageTags": tags}),
            )
            .unwrap()
        };

        assert_eq!(
            api.logo_url(&item(serde_json::json!({"Primary": "p"}))),
            None
        );
        assert_eq!(
            api.logo_url(&item(serde_json::json!({"Logo": "l"})))
                .as_deref(),
            Some("http://s/Items/i/Images/Logo?maxWidth=800&quality=90&tag=l")
        );
    }

    #[::core::prelude::v1::test]
    fn blurred_adds_server_blur_to_image_url() {
        assert_eq!(
            blurred("http://s/Items/x/Images/Primary?fillWidth=320&tag=t"),
            "http://s/Items/x/Images/Primary?fillWidth=320&tag=t&blur=30"
        );
    }
}
