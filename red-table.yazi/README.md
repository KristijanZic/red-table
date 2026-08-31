# red-table.yazi

Use red-table as a visual image selector inside Yazi. The plugin opens red-table
on Yazi's current real directory,
temporarily hands it the terminal, and replaces Yazi's selection with the
confirmed image paths.

## Requirements

- Unix; red-table result sessions currently use `/dev/tty`
- Yazi 26.5.6 or newer
- the `red-table` executable in `PATH`, or an absolute executable configured in
  `init.lua`

The plugin is a launcher and result bridge. It does not bundle the Rust binary.
It runs `red-table --select --print0 -- <current-directory>` directly, without a
shell.
It follows both the Yazi 26.5.6 URL-selection schema and the file-object schema
used by current Yazi releases.

## Install

When this repository is available from GitHub, install its subpackage with:

```sh
ya pkg add OWNER/red-table:red-table
```

Replace `OWNER` with the repository owner. Yazi records and locks the package in
its `package.toml`.

For a local checkout, link or copy this complete directory to:

```text
~/.config/yazi/plugins/red-table.yazi/
```

The repository flake also exposes the directory as `packages.yazi-plugin`. A
Home Manager configuration can use both outputs without relying on `PATH`:

```nix
programs.yazi = {
  enable = true;
  plugins.red-table = red-table.packages.${pkgs.system}.yazi-plugin;
  initLua = ''
    require("red-table"):setup({
      command = "${red-table.packages.${pkgs.system}.default}/bin/red-table",
    })
  '';
};
```

Here `red-table` is the name assigned to this repository's flake input.

## Bind a key

Add a binding to `~/.config/yazi/keymap.toml`:

```toml
[[mgr.prepend_keymap]]
on   = [ "g", "i" ]
run  = "plugin red-table"
desc = "Select images with red-table"
```

Press `g`, then `i`. In red-table, Space marks an image and `Ctrl+s` confirms.
`q` cancels.

## Executable configuration

If `red-table` is not in `PATH`, add this to `~/.config/yazi/init.lua`:

```lua
require("red-table"):setup({
  command = "/absolute/path/to/red-table",
})
```

The command is executed directly; it is not parsed by a shell. The configured
value must therefore name one executable and cannot contain shell arguments.

## Result behavior

- Confirmation replaces Yazi's existing selection and reveals the first result.
- Confirming no marks clears Yazi's selection.
- Cancellation (red-table status 2) leaves Yazi unchanged.
- Startup failures, malformed results, and other exit statuses create an error
  notification without changing the selection.
- Virtual/search directories are rejected; open a real directory before running
  the plugin.

Paths use red-table's NUL-delimited output. Filenames containing newlines or
non-UTF-8 bytes are not passed through a shell or line parser.

## Development

From the repository flake environment:

```sh
task test
task format-check
```

The Lua harness mocks Yazi's terminal, process, URL, filesystem, notification,
and manager APIs for both supported selection schemas. A PTY smoke test also
launches the flake's real Yazi 26.5.6-or-newer package, invokes red-table, confirms
an empty selection, and verifies successful return to Yazi.

## License

MIT. See `LICENSE` in this package directory.
