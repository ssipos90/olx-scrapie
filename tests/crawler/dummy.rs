use crate::helpers::spawn_app;
use httpmock::MockServer;
use olx_scrapie::config::TEST_ASSETS_DIR;

#[tokio::test]
async fn dummy() {
    let app = spawn_app().await;

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.path(app.config.list_page_url);
        then
            .body_from_file(format!("{}/{}", TEST_ASSETS_DIR, "grid-list-page.html"));
    });
    assert!(true);
}
