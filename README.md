# lazy_copy

A replacement for `io::copy` that avoids writing to the destination `File` until differing bytes are encountered.

## Purpose
`lazy_copy::copy` is meant to be used when you have a source of bytes that isn't a `File` and a destination that is a `File`. If you do not fall into this case, then other methods will probably be more efficient. For example if you have two files, then `fs::copy` will probably yield better results.