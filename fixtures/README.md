All .bin files are just random bytes with the exception of test1.bin and test2.bin

These were constructed to have a matching start, but differing tails.
```
$ cat fixtures/start.bin fixtures/end1.bin > fixtures/test1.bin
$ cat fixtures/start.bin fixtures/end2.bin > fixtures/test2.bin
```
