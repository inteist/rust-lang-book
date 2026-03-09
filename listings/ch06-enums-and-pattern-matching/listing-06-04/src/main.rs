// ANCHOR: here
// so we can inspect the state in a minute
#[derive(Debug)]
enum UsState {
    Alabama,
    Alaska,
    // --snip--
}

enum Coin {
    Penny,
    Nickel,
    Dime,
    Quarter(UsState),
}
// ANCHOR_END: here

fn main() {}
