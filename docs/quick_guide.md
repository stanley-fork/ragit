## 1. Create Knowledge-Base

First, let's say there're text files explaining ai. We'll build a knowledge-base on the text files. The dir should look like below.

```
ai_tutorials/
 ├ ai_tutorial_1.txt
 ├ ai_tutorial_2.txt
 ├ ai_tutorial_3.txt
 ╰ ... and many more txt files
```

Run `cd ai_tutorial; rag init`. You'll see a new dir created like below.

```
ai_tutorials/
 ├ .ragit/
 │  ├ chunks/
 │  ├ configs/
 │  ├ prompts/
 │  ╰ index.json
 ├ ai_tutorial_1.txt
 ├ ai_tutorial_2.txt
 ├ ai_tutorial_3.txt
 ╰ ... and many more txt files
```

`.ragit/` is like `.git/` of git repositories. It saves metadata and chunks. After `rag init`, the knowledge-base is empty. You have to add files to the staging using `rag add` command.

Run `rag add *.txt`. Now you're ready to build the knowledge-base. Run `rag build` to start the work. The default model is `llama3.1-70b-groq` and you need `GROQ_API_KEY` to run. If you want to run gpt-4o-mini, run `rag config --set model gpt-4o-mini`. You can see the list of the models using `rag ls-models`.

```
staged files: 5, processed files: 3
chunks: 3, chunk files: 1
curr processing file: ai_tutorial_1.txt
model: gpt-4o-mini
input tokens: 2081 (0.000$), output tokens: 327 (0.000$)
```

`rag build` takes very long time and money (if you're using proprietary api). It creates chunks and add title and summary to each chunk, using AI.

You can press Ctrl+C to pause the process. You can resume from where you left off by running `rag build` again. (more on [a dedicated document](./commands/build.txt))

```
ai_tutorials/
 ├ .ragit/
 │  ├ chunks/
 │  │  ╰ ... a lot of files ...
 │  ├ configs/
 │  ├ prompts/
 │  ╰ index.json
 ├ ai_tutorial_1.txt
 ├ ai_tutorial_2.txt
 ├ ai_tutorial_3.txt
 ╰ ... and many more txt files
```

After it's built, you'll see many data files in the `.ragit/` directory. You can ask queries on the knowledge-base now. Here are brief explanations on data files:

1. `/chunks` directory
2. `/configs` directory
3. `/logs` directory
4. `/prompts` directory
5. `usages.json` file

NOTE: You can ask queries on an incomplete knowledge-base, too.

## 2. (Optional) Pull Knowledge-Bases from web

This is the key part. You can download knowledge-bases from the internet and extend your knowledge-base with those. You can also share your knowledge-base with others.

First, let's make a fresh dir. Run `mkdir playground; cd playground`.

```
playground
```

Before downloading knowledge-bases, we have to init a rag index. Run `rag init`.

```
playground
 ╰ .ragit
    ├ chunks/
    ├ configs/
    ├ prompts/
    ╰ index.json
```

You'll see an empty rag index. Now we have to download knowledge-bases from the web. I have uploaded a few sample knowledge-bases for you. You can `rag clone` them, like `rag clone http://TODO/TODO`

- [docker](TODO)
- [git](TODO)
- [kubernetes](TODO)
- [nix](TODO)
- [postgresql](TODO)
- [ragit](TODO)
- [rustc-dev-guide](TODO)

```
playground
 ├ .ragit
 │  ├ chunks/
 │  ├ configs/
 │  ├ prompts/
 │  ╰ index.json
 ├ git
 │  ╰ .ragit
 │     ├ chunks/
 │     ├ configs/
 │     ├ prompts/
 │     ╰ index.json
 ╰ rustc-dev-guide
    ╰ .ragit
       ├ chunks/
       ├ configs/
       ├ prompts/
       ╰ index.json
```

Now we have 1 empty knowledge-base and 2 complete knowledge-bases in the playground. We're gonna use the empty knowledge-base as the main one. Let's extend the empty one. Run `rag ext ./git` and `rag ext ./rustc-dev-guide`.

## 3. Change Configs

Before asking a question or building a knowledge-base, you may want to change configurations. Configurations are very important because most commands cost money and you can optimize it with proper configurations.

Run `rag config --get model`. You'll see which model is used to answer your queries and build a knowledge-base.

Let's say you have free credits for Anthropic. By running `rag config --set model sonnet`, you can change your default model.

Run `rag config --get-all` to see all the keys and values.

## 4. Ask questions on a Knowledge-Base

Asking query is straight forward: `rag query "Tell me how the rust compiler uses git"`

If you want an interactive chat, run `rag query --interactive`.
