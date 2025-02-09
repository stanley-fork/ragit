# How ragit works

Ragit's RAG pipeline is a bit different from other RAG frameworks. It does not use embeddings (I'm planning to implement ones, tho), and 

1. User inputs a query.
2. LLM extracts search-keywords from the query.
  - For example, if the query is "How does this server implement user authentication?", the keywords would be "server, authentication, login, 2fa, jwt".
3. It runs tfidf-search based on the keywords from step 2. It then retrieves top 10 chunks. (the number is configurable)
4. It reranks the top 10 chunks from step 3. It selects the 3 most relevant chunks. (the number is configurable)
5. It RAGs based on the 3 chunks from step 4.

TODO: more detailed documentation