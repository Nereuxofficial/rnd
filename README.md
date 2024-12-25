# RND

> A wayland notification daemon written in Rust with iced.

## Running RND

1. Kill your existing Notification Daemon

```bash
# For Plasma
systemctl --user stop plasmashell # You can start plasmashell again by running this command with `start` instead of `stop`
```

2. Clone the repository and run rnd:

```bash
git clone https://github.com/Nereuxofficial/rnd
cd rnd
cargo run -r
```