# abbs

`abbs` is a modern ssh BBS system based on [`ssh_ui`](https://github.com/ellenhp/ssh_ui). Currently it allows users to visit a library provided by a ZIM file. ZIM blobs are searchable with [`tantivy`](https://github.com/quickwit-oss/tantivy), both by title and content. Prefix search is supported.

### Roadmap

* SSH public key authentication
* Account recovery via `https://github.com/{handle}.keys` or any user-defined HTTPS endpoint.
* Basic forum system with users, posts and threads.
* Support for door games (probably only available on FreeBSD via jails, unless there's a good way to sandbox things on Linux?).