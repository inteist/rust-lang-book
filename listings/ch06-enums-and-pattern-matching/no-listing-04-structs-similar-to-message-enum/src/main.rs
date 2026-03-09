// ANCHOR: here
// unit struct
struct QuitMessage;
struct MoveMessage {
    x: i32,
    y: i32,
}
// tuple struct
struct WriteMessage(String);
// tuple struct
struct ChangeColorMessage(i32, i32, i32);
// ANCHOR_END: here

fn main() {}
