yammer
======

Yammer provides asynchronous bindings to the Ollama API and the following CLI tools:

- `shellm` pass a file (or stdin if no file) to the generate endpoint and stream the result.
- `oneshot` open a temporary file in an editor to be passed to the generate endpoint; stream the
  result.
- `prompt` pass a prompt to the generate endpoint and stream the result.
- `chat` chat with a model using the chat endpoint.
- `chats` manage chat sessions.

Installation
------------

```sh
$ cargo install yammer
```

Usage
-----

The shellm tool multiplexes files over a model:

```sh
$ shellm --model llama3.2:3b << EOF
Why is the sky red?
EOF
I'm sorry.  The sky is not red.
$ shellm --model llama3.2:3b foo bar
Response to foo...
Response to bar...
```

The oneshot tool is conceptually the same as editing a temporary file and passing it to shellm:

```sh
$ oneshot llama3.2:3b gemma2
Opens $EDITOR with a temporary file.  Write your prompt and save the file.
Output of llama3.2:3b...
Output of gemma2....
```

The prompt tool is similar to shellm but takes prompts on the command line rather than files:

```sh
$ prompt llama3.2:3b "Why is the sky red?"
I'm sorry.  The sky is not red.
```

The chat command is used to chat with a model:

```sh
$ chat
>>> Why is the sky red?
The sky often appears red at sunrise and sunset. ...
>>> :edit
>>> :model llama3.2:3b
>>> :retry
The sky often appears red at sunrise and sunset due to Rayleigh scattering. ....
>>> :param --num-ctx 4096
>>> :exit
```

The chats command is used to manage chat sessions:

```sh
$ chats
recent:
2024-12-01T18:26 FP8MC gemma2              Why is the sky red?
2024-12-01T17:34 H5HMV llama3.2:3b         Hi there!  Tell me about first and follow sets for parsers.
> pin FP8MC
> status
pinned:
2024-12-01T18:29 FP8MC gemma2              Why is the sky red?

recent:
2024-12-01T17:34 H5HMV llama3.2:3b         Hi there!  Tell me about first and follow sets for parsers.
> archive H5HMV
> status
pinned:
2024-12-01T18:29 FP8MC gemma2              Why is the sky red?
> chat FP8MC
>>> Why is the sky red?
The sky often appears red at sunrise and sunset. ...
>>> exit
> new "Act like Mario, the video game character."
>>> Hi!
Hiya!  It'sa me, Mario!
```

Status
------

Active development.

Documentation
-------------

The latest documentation is always available at [docs.rs](https://docs.rs/yammer/latest/yammer/).
