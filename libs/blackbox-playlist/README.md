# blackbox-playlist

M3U and M3U8 playlists and the choice of files in a folder. No file access and no game knowledge: the caller reads
the bytes and the names, and gets back entries, resolved paths and the files it may play.

- `parse`: the entries of an M3U text, with the `#EXTINF` length and caption (`Artist - Title`). The text is UTF-8
  (a byte-order mark is dropped) or, when that fails, Latin-1.
- `resolve`: the path an entry names, relative to the playlist's folder (either slash) or absolute.
- `select`: the files whose extension matches, sorted by file name.

License: MIT OR Apache-2.0.
