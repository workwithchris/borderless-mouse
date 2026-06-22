use std::time::Duration;

use bm_core::protocol::{Event, MouseButton, PROTOCOL_VERSION};
use bm_core::transport::{bind, Connection};
use tokio::net::TcpListener;
use tokio::time::timeout;

/// Full handshake integration test:
/// Client connects -> sends Hello -> Server responds HelloAck -> bidirectional event exchange.
#[tokio::test]
async fn full_handshake_and_event_stream() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = tokio::spawn(async move {
        let (stream, _peer) = listener.accept().await.unwrap();
        let mut conn = Connection::from_stream(stream).unwrap();

        // Read Hello
        let event = conn.read().await.unwrap().unwrap();
        let (hostname, version) = match &event {
            Event::Hello { hostname, version, .. } => (hostname.clone(), *version),
            other => panic!("expected Hello, got {other:?}"),
        };
        assert_eq!(version, PROTOCOL_VERSION, "client {hostname} has wrong version");

        // Send HelloAck
        conn.write(&Event::HelloAck {
            version: PROTOCOL_VERSION,
            hostname: "server-host".into(),
            display_size: (2560, 1440),
        })
        .await
        .unwrap();

        // Receive a MouseMove
        let event = conn.read().await.unwrap().unwrap();
        match event {
            Event::MouseMove { x, y } => {
                assert!((x - 100.0).abs() < f64::EPSILON);
                assert!((y - 200.0).abs() < f64::EPSILON);
            }
            other => panic!("expected MouseMove, got {other:?}"),
        }

        // Send a MouseMove back
        conn.write(&Event::MouseMove { x: 300.0, y: 400.0 })
            .await
            .unwrap();

        // Receive Disconnect
        let event = conn.read().await.unwrap().unwrap();
        match event {
            Event::Disconnect { reason } => {
                assert_eq!(reason, "test complete");
            }
            other => panic!("expected Disconnect, got {other:?}"),
        }
    });

    let client_handle = tokio::spawn(async move {
        let mut conn = Connection::connect(&addr.to_string()).await.unwrap();

        // Send Hello
        conn.write(&Event::Hello {
            version: PROTOCOL_VERSION,
            hostname: "client-host".into(),
            display_size: (1920, 1080),
        })
        .await
        .unwrap();

        // Receive HelloAck
        let event = conn.read().await.unwrap().unwrap();
        match event {
            Event::HelloAck { version, hostname, .. } => {
                assert_eq!(version, PROTOCOL_VERSION);
                assert_eq!(hostname, "server-host");
            }
            other => panic!("expected HelloAck, got {other:?}"),
        }

        // Send MouseMove
        conn.write(&Event::MouseMove { x: 100.0, y: 200.0 })
            .await
            .unwrap();

        // Receive MouseMove back
        let event = conn.read().await.unwrap().unwrap();
        match event {
            Event::MouseMove { x, y } => {
                assert!((x - 300.0).abs() < f64::EPSILON);
                assert!((y - 400.0).abs() < f64::EPSILON);
            }
            other => panic!("expected MouseMove, got {other:?}"),
        }

        // Send Disconnect
        conn.write(&Event::Disconnect {
            reason: "test complete".into(),
        })
        .await
        .unwrap();
    });

    timeout(Duration::from_secs(5), async {
        server_handle.await.unwrap();
        client_handle.await.unwrap();
    })
    .await
    .expect("handshake test timed out");
}

/// Test Ping-Pong roundtrip latency check.
#[tokio::test]
async fn ping_pong_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut conn = Connection::from_stream(stream).unwrap();
        let event = conn.read().await.unwrap().unwrap();
        if let Event::Ping(id) = event {
            conn.write(&Event::Pong(id)).await.unwrap();
        }
    });

    let client = tokio::spawn(async move {
        let mut conn = Connection::connect(&addr.to_string()).await.unwrap();
        conn.write(&Event::Ping(42)).await.unwrap();
        let event = conn.read().await.unwrap().unwrap();
        assert!(matches!(event, Event::Pong(42)));
    });

    timeout(Duration::from_secs(5), async {
        server.await.unwrap();
        client.await.unwrap();
    })
    .await
    .expect("ping-pong timed out");
}

/// Test disconnect notification is received.
#[tokio::test]
async fn disconnect_notification() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut conn = Connection::from_stream(stream).unwrap();
        let event = conn.read().await.unwrap().unwrap();
        match event {
            Event::Disconnect { reason } => {
                assert_eq!(reason, "goodbye");
            }
            other => panic!("expected Disconnect, got {other:?}"),
        }
    });

    let client = tokio::spawn(async move {
        let mut conn = Connection::connect(&addr.to_string()).await.unwrap();
        conn.write(&Event::Disconnect {
            reason: "goodbye".into(),
        })
        .await
        .unwrap();
    });

    timeout(Duration::from_secs(5), async {
        server.await.unwrap();
        client.await.unwrap();
    })
    .await
    .expect("disconnect test timed out");
}

/// Test sending multiple events sequentially.
#[tokio::test]
async fn multiple_events_batch() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let n_events = 500;

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut conn = Connection::from_stream(stream).unwrap();
        for i in 0..n_events {
            let event = conn.read().await.unwrap().unwrap();
            match event {
                Event::Ping(id) => assert_eq!(id, i),
                other => panic!("expected Ping({i}), got {other:?}"),
            }
        }
    });

    let client = tokio::spawn(async move {
        let mut conn = Connection::connect(&addr.to_string()).await.unwrap();
        for i in 0..n_events {
            conn.write(&Event::Ping(i)).await.unwrap();
        }
    });

    timeout(Duration::from_secs(10), async {
        server.await.unwrap();
        client.await.unwrap();
    })
    .await
    .expect("batch test timed out");
}

/// Test interleaved two-way communication.
#[tokio::test]
async fn bidirectional_events() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut conn = Connection::from_stream(stream).unwrap();

        let event = conn.read().await.unwrap().unwrap();
        assert!(matches!(event, Event::Ping(1)));
        conn.write(&Event::Pong(1)).await.unwrap();
        conn.write(&Event::Ping(2)).await.unwrap();

        let event = conn.read().await.unwrap().unwrap();
        assert!(matches!(event, Event::Pong(2)));
    });

    let client = tokio::spawn(async move {
        let mut conn = Connection::connect(&addr.to_string()).await.unwrap();
        conn.write(&Event::Ping(1)).await.unwrap();

        let event = conn.read().await.unwrap().unwrap();
        assert!(matches!(event, Event::Pong(1)));

        let event = conn.read().await.unwrap().unwrap();
        assert!(matches!(event, Event::Ping(2)));
        conn.write(&Event::Pong(2)).await.unwrap();
    });

    timeout(Duration::from_secs(5), async {
        server.await.unwrap();
        client.await.unwrap();
    })
    .await
    .expect("bidirectional test timed out");
}

/// Test various event types can be sent over the wire.
#[tokio::test]
async fn all_event_types_over_wire() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let events_to_send = vec![
        Event::Hello {
            version: 1,
            hostname: "test".into(),
            display_size: (1920, 1080),
        },
        Event::HelloAck {
            version: 1,
            hostname: "server".into(),
            display_size: (2560, 1440),
        },
        Event::MouseMove { x: 100.0, y: 200.0 },
        Event::MouseMoveRel { dx: -5.0, dy: 3.0 },
        Event::MouseButton {
            button: MouseButton::Right,
            pressed: true,
        },
        Event::MouseScroll { dx: 0.0, dy: -3.0 },
        Event::KeyEvent {
            keycode: 42,
            pressed: true,
            modifiers: 2,
        },
        Event::ClipboardChanged {
            content: "clipboard text".into(),
        },
        Event::CursorEnter,
        Event::CursorLeave,
        Event::Disconnect {
            reason: "test end".into(),
        },
    ];

    let events_clone = events_to_send.clone();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut conn = Connection::from_stream(stream).unwrap();
        for expected in &events_to_send {
            let received = conn.read().await.unwrap().unwrap();
            assert_eq!(
                serde_json::to_value(&received).unwrap(),
                serde_json::to_value(expected).unwrap(),
                "event mismatch"
            );
        }
    });

    let client = tokio::spawn(async move {
        let mut conn = Connection::connect(&addr.to_string()).await.unwrap();
        for event in &events_clone {
            conn.write(event).await.unwrap();
        }
    });

    timeout(Duration::from_secs(5), async {
        server.await.unwrap();
        client.await.unwrap();
    })
    .await
    .expect("all events test timed out");
}

/// Test that bind and connect helper functions work.
#[tokio::test]
async fn bind_and_connect_helpers() {
    let listener = bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut conn = Connection::from_stream(stream).unwrap();
        let event = conn.read().await.unwrap().unwrap();
        assert!(matches!(event, Event::Ping(99)));
    });

    let client = tokio::spawn(async move {
        let mut conn = Connection::connect(&addr.to_string()).await.unwrap();
        conn.write(&Event::Ping(99)).await.unwrap();
    });

    timeout(Duration::from_secs(5), async {
        server.await.unwrap();
        client.await.unwrap();
    })
    .await
    .expect("bind/connect helper test timed out");
}
