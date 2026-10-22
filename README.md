# file_server

An `http` file server written in rust using [tokio](https://tokio.rs/) and
[hyper](https://hyper.rs/).

Includes support for:
- http 1.1 / 2
- boxed responses (send large files frame by frame)
- `head` requests
- `range` requests
- encoded requests

## How to use

### Install

Bash the following commands:

```sh
git clone https://github.com/herebythere/file_server
cargo install --path file_server/file_server
```

### Run

Bash the following command:

```sh
file_server
```

This will start `file_server` with using the `cwd` as a base directory.

Now files can be requested from the `cwd` at `localhost:3000`:

```sh
curl localhost:3000
```

### Configuration

The following command-line arguments are valid paramters: 

```sh
file_server /
	--host=0.0.0.0 /
	--port=4000 /
	--directory=../another_directory /
	--content-encodings=gzip,deflate,br,zstd
```

### Accept-Encoding

When a request has an `accept-encoding` header, `file_server` will return a corresponding `zip`-ed version of file if available.

So if a request has the following header:

```
Accept-Encoding: gzip;
```

And the target file has a correspponding gziped file: 

```sh
index.html		# source file
index.html.gz	# gzipped file
```

`File_server` will send the encoded file, if available. Otherwise, it serves the source file.

### Dynamic encoding not supported

`File_server` does not encode or zip files ever.

This program serves static files. Just zip them up now to save memory resources.

### Range requests

`File_server` supports single range requests.

Bash the following command:

```sh
curl -v -r 0-6 localhost:3000
```

And the first 6 bytes of `index.html` will be delivered.

Multipart ranges are not currently supported.

They're a little more strenuous, tend to hog memory, and are difficult to deliver efficiently. Sorry.

Valid range sytanxes:
```
<unit>=<range-start>-
<unit>=<range-start>-<range-end>
<unit>=-<suffix-length>
```

### Serialize details

Details for a particular file or directory can be requested with some url params:

```
http://127.0.0.1:3000/?serialize_as=json
```

## License

`file_server` is released under the BSD 3-Clause License.
