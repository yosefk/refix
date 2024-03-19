Here's [the long story](https://yosefk.com/blog/refix-fast-debuggable-reproducible-builds.html) about what refix does and why you'd want to do this.

The short story is, `refix` **re**places **fix**ed-size string p**refix**es inside binary files (executables, shared libraries, static libraries, and object files.) It can also replace
the data of whole sections with equally sized data read from a file (if `--section name file` is passed one or more times.)

Why would you want this? Let's say you have reproducible builds, which have relative paths to source code, and no build-specific information
(like the absolute path of the final executable, the build time etc.) Then you might want - I'd say you *very much do want* - to put
the absolute source code paths, and all that build-specific information, into the executable once it leaves the build system cache
(where only reproducible artifacts belong) and is delivered to your filesystem. You want to _refix_ the binary back to the original
source path & other build context, as it were, after the build system detached it from this context. Putting this info back helps debugging a lot!
You don't want finding the source code in debuggers and other tools to be a puzzle; you want it to just work.

= Putting absolute source file paths into your binaries =

* Run with the equivalent of the following `gcc` flags:
  <pre>
  -fdebug-prefix-map==MAGIC <i># for DWARF</i>
  -ffile-prefix-map==MAGIC  <i># for __FILE__</i>
  </pre>
* Make MAGIC long enough for any source path prefix you're willing to support.
* Why the `==` in the flag? This invocation assumes that file paths
    are relative, so it remaps _the empty string_ to MAGIC, meaning, `dir/file.c` becomes `MAGICdir/file.c`.
    You can also pass `=/prefix/to/remap=MAGIC`, if your build system uses absolute paths.
* After the linked binary is delivered, use `refix` to put the actual source path back in (this basically works
  like replacing with `sed` would, but taking tens of milliseconds instead of seconds):
  <pre>refix binary MAGIC actual-source-path</pre>
* If the source path is shorter than the length of MAGIC, pad it with forward slashes: `/////home/user/src/`.
  If the source path is too long, the post-link step should truncate it, warn, and eventually be changed to outright fail.

That's it, now all debugging tools will find the source code effortlessly!

= Putting other build-specific info into your binaries =

  
