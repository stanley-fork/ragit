# How ragit works

Ragit's RAG pipeline is a bit different from other RAG frameworks. It does not use embeddings (I'm planning to implement ones, tho), and 

1. User inputs a query.
2. LLM extracts search-keywords from the query.
  - For example, if the query is "How does this server implement user authentication?", the keywords would be "server, authentication, login, 2fa, jwt".
3. It runs tfidf-search based on the keywords from step 2. It then retrieves top 10 chunks. (the number is configurable)
4. It reranks the top 10 chunks from step 3. It selects the 3 most relevant chunks. (the number is configurable)
5. It RAGs based on the 3 chunks from step 4.

## Step 1

> User inputs a query.

## Step 2

> LLM extracts search-keywords from the query. Ragit uses tfidf-scores instead of embedding vectors. Unlike embeddings, tfidf is not suitable for query <-> chunk matching.

Let's say the query is "How does this server implement user authentication?". The words "How", "does" and "this" are not very helpful for the tfidf scoring. There must be tons of irrelevant chunks that contain "does". Also, relevant chunks are likely to contain words like "JWT", "login" or "2fa". LLMs are smart enough to know this. LLMs transform a natural language query `"How does this server implement user authentication?"` to a list of strings `["server", "authentication", "2fa", "login", "jwt"]`. This list will be used for tfidf-scoring.

If you want to simulate step 2, use `rag extract-keywords` command.

## Step 3

> It runs tfidf-search based on the keywords from step 2. It then retrieves top 10 chunks. (the number is configurable)

Run `rag config --set max_summaries 20` to retrieve 20 chunks from tfidf-scoring.

## Step 4

> It reranks the top 10 chunks from step 3. It selects the 3 most relevant chunks. (the number is configurable)

Step 3 retrieved 10 related chunks, but many of them are irrelevant. Tfidf is not that strong. In this step, LLM reviews titles and summaries of all the 10 chunks and selects the most relevant 3 chunks.

Run `rag config --set max_retrieval 5` to select 5 most relevant chunks.

If you want to simulate step 2, 3 and 4, use `rag retrieve-chunks` command.

## Step 5

> It RAGs based on the 3 chunks from step 4.
