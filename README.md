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
>>> exit
> exit
```

Help
----

### shellm

```sh
$ shellm --help
USAGE: shellm [OPTIONS] [FILE]

Options:
    -h, -help           Print this help menu.
        -ollama-host    The host to connect to.
        -model          The model to use from the ollama library.
        -suffix         The suffix to append to the response.
        -system         The system to use in the template.
        -template       The template to use for the prompt.
        -json           Format the response in JSON. You must also ask the
                        model to do so.
        -raw            Whether to pass bypass formatting of the prompt.
        -keep-alive     Duration to keep the model in memory for after the
                        call.
        -param-mirostat
                        Enable Mirostat sampling for controlling perplexity.
                        (default: 0, 0 = disabled, 1 = Mirostat, 2 = Mirostat
                        2.0)
        -param-mirostat-eta
                        Influences how quickly the algorithm responds to
                        feedback from the generated text.
        -param-mirostat-tau
                        Controls the balance between coherence and diversity
                        of the output.
        -param-num-ctx  The number of tokens worth of context to allocate.
        -param-repeat-last-n
                        Sets how far back for the model to look back to
                        prevent repetition.
        -param-repeat-penalty
                        Sets how strongly to penalize repetitions.
        -param-temperature
                        The temperature of the model.
        -param-seed     Sets the random number seed to use for generation.
        -param-tfs-z    Tail free sampling is used to reduce the impact of
                        less probable tokens from the output.
        -param-num-predict
                        Maximum number of tokens to predict when generating
                        text.
        -param-top-k    Reduces the probability of generating nonsense.
        -param-top-p    Works together with top-k.
        -param-min-p    Alternative to the top_p, and aims to ensure a balance
                        of quality and variety.
```

### oneshot

```sh
$ oneshot --help
USAGE: oneshot [OPTIONS] [MODEL]

Options:
    -h, -help           Print this help menu.
        -ollama-host    The host to connect to.
        -suffix         The suffix to append to the response.
        -system         The system to use in the template.
        -template       The template to use for the prompt.
        -json           Format the response in JSON. You must also ask the
                        model to do so.
        -raw            Whether to pass bypass formatting of the prompt.
        -keep-alive     Duration to keep the model in memory for after the
                        call.
        -param-mirostat
                        Enable Mirostat sampling for controlling perplexity.
                        (default: 0, 0 = disabled, 1 = Mirostat, 2 = Mirostat
                        2.0)
        -param-mirostat-eta
                        Influences how quickly the algorithm responds to
                        feedback from the generated text.
        -param-mirostat-tau
                        Controls the balance between coherence and diversity
                        of the output.
        -param-num-ctx  The number of tokens worth of context to allocate.
        -param-repeat-last-n
                        Sets how far back for the model to look back to
                        prevent repetition.
        -param-repeat-penalty
                        Sets how strongly to penalize repetitions.
        -param-temperature
                        The temperature of the model.
        -param-seed     Sets the random number seed to use for generation.
        -param-tfs-z    Tail free sampling is used to reduce the impact of
                        less probable tokens from the output.
        -param-num-predict
                        Maximum number of tokens to predict when generating
                        text.
        -param-top-k    Reduces the probability of generating nonsense.
        -param-top-p    Works together with top-k.
        -param-min-p    Alternative to the top_p, and aims to ensure a balance
                        of quality and variety.
```

### prompt

```sh
$ prompt --help
USAGE: prompt [OPTIONS] [PROMPT]

Options:
    -h, -help           Print this help menu.
        -ollama-host    The host to connect to.
        -model          The model to use from the ollama library.
        -suffix         The suffix to append to the response.
        -system         The system to use in the template.
        -template       The template to use for the prompt.
        -json           Format the response in JSON. You must also ask the
                        model to do so.
        -raw            Whether to pass bypass formatting of the prompt.
        -keep-alive     Duration to keep the model in memory for after the
                        call.
        -param-mirostat
                        Enable Mirostat sampling for controlling perplexity.
                        (default: 0, 0 = disabled, 1 = Mirostat, 2 = Mirostat
                        2.0)
        -param-mirostat-eta
                        Influences how quickly the algorithm responds to
                        feedback from the generated text.
        -param-mirostat-tau
                        Controls the balance between coherence and diversity
                        of the output.
        -param-num-ctx  The number of tokens worth of context to allocate.
        -param-repeat-last-n
                        Sets how far back for the model to look back to
                        prevent repetition.
        -param-repeat-penalty
                        Sets how strongly to penalize repetitions.
        -param-temperature
                        The temperature of the model.
        -param-seed     Sets the random number seed to use for generation.
        -param-tfs-z    Tail free sampling is used to reduce the impact of
                        less probable tokens from the output.
        -param-num-predict
                        Maximum number of tokens to predict when generating
                        text.
        -param-top-k    Reduces the probability of generating nonsense.
        -param-top-p    Works together with top-k.
        -param-min-p    Alternative to the top_p, and aims to ensure a balance
                        of quality and variety.
```

### chat

```sh
$ chat --help
USAGE: chat [OPTIONS]

Options:
    -h, -help           Print this help menu.
        -ollama-host    The host to connect to.
        -model          The model to use from the ollama library.
        -keep-alive     Duration to keep the model in memory for after the
                        call.
        -param-mirostat
                        Enable Mirostat sampling for controlling perplexity.
                        (default: 0, 0 = disabled, 1 = Mirostat, 2 = Mirostat
                        2.0)
        -param-mirostat-eta
                        Influences how quickly the algorithm responds to
                        feedback from the generated text.
        -param-mirostat-tau
                        Controls the balance between coherence and diversity
                        of the output.
        -param-num-ctx  The number of tokens worth of context to allocate.
        -param-repeat-last-n
                        Sets how far back for the model to look back to
                        prevent repetition.
        -param-repeat-penalty
                        Sets how strongly to penalize repetitions.
        -param-temperature
                        The temperature of the model.
        -param-seed     Sets the random number seed to use for generation.
        -param-tfs-z    Tail free sampling is used to reduce the impact of
                        less probable tokens from the output.
        -param-num-predict
                        Maximum number of tokens to predict when generating
                        text.
        -param-top-k    Reduces the probability of generating nonsense.
        -param-top-p    Works together with top-k.
        -param-min-p    Alternative to the top_p, and aims to ensure a balance
                        of quality and variety.
```

### chats

```sh
$ chats
> help
chats
=====

Commands:

status      Show the status of all chats.
archive     Archive a chat.
unarchive   Unarchive a chat.
archived    Show all archived chats.
pin         Pin a chat.
unpin       Unpin a chat.
pinned      Show all pinned chats.
new         Start a new chat.
chat        Continue a chat.
editor      Start a chat with a system message written in EDITOR.
```

Status
------

Active development.

Documentation
-------------

The latest documentation is always available at [docs.rs](https://docs.rs/yammer/latest/yammer/).
