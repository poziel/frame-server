use frame_terminal::Terminals;

fn main() {
    for profile in Terminals::default().profiles() {
        println!("{}: {}", profile.id, profile.path.display());
    }
}
