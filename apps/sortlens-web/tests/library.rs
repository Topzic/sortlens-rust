//! The wasm client's library path (SL-J02): a snapshot from the wire opens as an engine in
//! memory, ops apply with the same rules as the desktop, and `save` hands back the exact JSON
//! the next session opens. No server process here — the protocol itself is proven against the
//! real one in `apps/sortlens-server/tests/client.rs`.

use serde_json::json;
use sortlens_testkit::Fixture;
use sortlens_web::RemoteLibrary;

/// A scanned disk library's snapshot hydrates into a client that sees and operates on the
/// same photographs, without the directory, the lock or the log the disk side needs.
#[test]
fn a_snapshot_from_the_server_hydrates_into_a_working_library() {
    let fixture = Fixture::new("web-hydrate");
    fixture.jpeg("Photos/a.jpg", [210, 60, 40], 48, 32);
    fixture.jpeg("Photos/b.jpg", [40, 90, 210], 40, 40);
    let photos = fixture.root().join("Photos");
    let library_dir = fixture.root().join("library");
    let snapshot = {
        let mut engine = sortlens_engine::Engine::open(&library_dir).expect("the library opens");
        engine
            .run("library.addFolder", &json!({ "path": photos.to_string_lossy() }))
            .expect("scan starts");
        loop {
            if engine.poll().is_some() {
                continue;
            }
            if engine.scan_progress().is_some() {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            break;
        }
        engine.run("photo.rate", &json!({ "ids": [1], "stars": 4 })).expect("rate");
        let json = engine.snapshot_json().expect("snapshot json");
        engine.close().expect("close");
        json
    };

    // The page opens what the wire carried.
    let mut client = RemoteLibrary::hydrate(&snapshot).expect("hydrate");
    assert!(client.engine().is_memory());
    let page = client
        .engine()
        .page(&sortlens_engine::Filter::default(), sortlens_engine::Sort::Natural, 0, 10);
    assert_eq!(page.total, 2);
    assert_eq!(client.engine().photo(sortlens_core::PhotoId(1)).expect("photo").rating, 4);

    // The client annotates, undoes, and the state is what it says.
    client
        .engine_mut()
        .run("photo.rate", &json!({ "ids": [2], "stars": 5 }))
        .expect("rate in the client");
    assert_eq!(client.engine().photo(sortlens_core::PhotoId(2)).expect("photo").rating, 5);
    client.engine_mut().run("edit.undo", &json!({})).expect("undo");
    assert_eq!(client.engine().photo(sortlens_core::PhotoId(2)).expect("photo").rating, 0);

    // The save is the next session's open: the stars cross it intact.
    let saved = client.save().expect("save");
    let reopened = RemoteLibrary::hydrate(&saved).expect("reopen");
    assert_eq!(reopened.engine().photo(sortlens_core::PhotoId(1)).expect("photo").rating, 4);
}

/// A fresh client — the page before its first pairing — is an honest empty library, and its
/// own save reopens as the same empty state.
#[test]
fn a_fresh_client_starts_empty_and_survives_its_own_save() {
    let client = RemoteLibrary::fresh().expect("fresh");
    assert!(client.engine().is_memory());
    assert_eq!(
        client
            .engine()
            .page(&sortlens_engine::Filter::default(), sortlens_engine::Sort::Natural, 0, 10)
            .total,
        0
    );
    let saved = client.save().expect("save");
    let reopened = RemoteLibrary::hydrate(&saved).expect("reopen");
    assert_eq!(
        reopened
            .engine()
            .page(&sortlens_engine::Filter::default(), sortlens_engine::Sort::Natural, 0, 10)
            .total,
        0
    );
}

/// A snapshot the page cannot read is refused, not opened pretending: broken JSON and a
/// schema newer than this build both say so.
#[test]
fn a_snapshot_the_client_cannot_read_is_refused() {
    assert!(RemoteLibrary::hydrate("{not json").is_err());
    // One past this build's schema: structurally valid, deliberately unreadable.
    let fresh = RemoteLibrary::fresh().expect("fresh");
    let saved = fresh.save().expect("save");
    // One past this build's schema: structurally valid, deliberately unreadable. The catalog's
    // schema is 2 today; the number only has to be larger than it, and the parse-back assertion
    // proves the string really was mangled into that schema rather than into nonsense.
    let newer = saved.replacen("\"schema\":2", "\"schema\":3", 1);
    let value: serde_json::Value = serde_json::from_str(&newer).expect("still json");
    assert_eq!(value["schema"], 3);
    let error = match RemoteLibrary::hydrate(&newer) {
        Ok(_) => panic!("a newer schema opened"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("newer than"), "the refusal names the problem: {error}");
}
