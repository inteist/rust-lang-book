// required for mdbook test
extern crate trpl;

fn main() {
    trpl::block_on(async {
        // ANCHOR: channel
        let (tx, mut rx) = trpl::channel();

        let val = String::from("hi");
        tx.send(val).unwrap();

        let received = rx.recv().await.unwrap();
        println!("received '{received}'");
        // ANCHOR_END: channel
    });
}
