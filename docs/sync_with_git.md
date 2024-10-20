# Sync with git

TODO: `rag sync --git` is not implemented yet

If the files in your knowledge-base are tracked by git, congratulations! As its name suggests, "ragit" integrates very well with git.

Let's first clone a repository. Run `git clone https://github.com/baehyunsol/ragit`.

Then, run `cd ragit; rag init` to intialize a knowledge-base.

All you have to do now is `rag sync --git` and `rag build`. `rag sync --git` is simply an alias for `rag remove --auto; rag add --auto --git`. When you first run this, it'll add all the files in the git's working tree to ragit's staging area. `rag build` will build the knowledge-base. That's it. Now you can ask questions on the repo.

Let's say there has been updates on the remote repository. You can easily apply the updates. First run `git pull` to update the files. Then, run `rag sync --git; rag build` again. That's it!
