# Clean Recently Used

Linux desktop systems maintain a list of recently used files.
This program lets you filter that list, removing all entries below one or more directories.

## One-time usage

To remove all entries below `/tmp` or `/var/tmp` from the list, run:

```sh
cargo run /tmp /var/tmp
```

## Periodic usage

This repository includes a pair of systemd user units that can be used to clean the list periodically.
Install them with:

```sh
make install
systemctl --user daemon-reload
```

Then enable the timer for a certain path like so:

```sh
systemctl --user enable --now "clean-recently-used@$(systemd-escape -p /tmp).timer"
```

This will clean `/tmp` from the recently-used file every hour.
(If you want to clean more than one directory at once, you will need to write a custom service unit.)

## License

[Blue Oak Model License 1.0.0](./LICENSE.md).
