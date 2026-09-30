use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
};

use speare_legacy_account_import::{HttpLegacySource, LegacySource};

#[test]
fn configurable_prefix_and_contract_queries_are_exact() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sent, received) = mpsc::channel();
    let server = thread::spawn(move || {
        let bodies = [
            r#"{"datasets":[],"pagination":{"default_limit":100,"max_limit":1000,"continuation":"exclusive after_id"},"consistency":"application-quiesced"}"#,
            r#"{"dataset":"Card","rows":[],"next_after_id":null,"complete":true}"#,
            r#"{"dataset":"Card","rows":[],"next_after_id":null,"complete":true}"#,
        ];
        for body in bodies {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let read = socket.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            sent.send(request.lines().next().unwrap().to_owned())
                .unwrap();
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });

    let source = HttpLegacySource::with_endpoint_prefix(
        &format!("http://{address}/"),
        "/api/migration/export/v1",
        "test-token",
    )
    .unwrap();
    source.manifest().unwrap();
    source.page("Card", None, 7).unwrap();
    source.page("Card", Some(-3), 7).unwrap();
    server.join().unwrap();

    assert_eq!(
        received.into_iter().collect::<Vec<_>>(),
        [
            "GET /api/migration/export/v1/manifest HTTP/1.1",
            "GET /api/migration/export/v1/datasets/Card?limit=7 HTTP/1.1",
            "GET /api/migration/export/v1/datasets/Card?after_id=-3&limit=7 HTTP/1.1",
        ]
    );
}
