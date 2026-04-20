use binance_rs::api::announcements::{AnnouncementListRequest, BinanceAnnouncements};
use binance_rs::client::BinanceClient;
use mockito::{Matcher, Server};

#[tokio::test]
async fn announcements_wrapper_maps_binance_website_endpoint() {
    let mut server = Server::new_async().await;
    let announcements = server
        .mock("GET", "/bapi/composite/v1/public/cms/article/list/query")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("type".into(), "1".into()),
            Matcher::UrlEncoded("catalogId".into(), "48".into()),
            Matcher::UrlEncoded("pageNo".into(), "2".into()),
            Matcher::UrlEncoded("pageSize".into(), "20".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"000000","data":{"articles":[{"code":"abc","title":"Listing Notice"}]}}"#,
        )
        .create_async()
        .await;

    let mut client = BinanceClient::new_public().unwrap();
    client.set_base_url(server.url());
    let api = BinanceAnnouncements::new(client);

    let response = api
        .get_announcements(
            AnnouncementListRequest::new()
                .with_catalog_id(48)
                .with_page(2)
                .with_page_size(20),
        )
        .await
        .unwrap();

    assert_eq!(response["data"]["articles"][0]["title"], "Listing Notice");
    announcements.assert_async().await;
}
