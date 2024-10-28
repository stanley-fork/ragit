import re
from selenium import webdriver
from selenium.webdriver.common.by import By
import sys
import time
from typing import Optional

def generate_link(title: str) -> str:
    title = title.replace(" ", "_")
    return f"https://en.wikipedia.org/w/index.php?title={title}&action=edit"

def generate_synonym_link(title: str) -> str:
    title = title.replace(" ", "_")
    return f"https://en.wikipedia.org/wiki/Special:WhatLinksHere?target={title}&limit=100&namespace=&hidetrans=1&hidelinks=1"

def clean_text(text: str) -> dict:
    result = {}

    while (r := re.search(r"{{([a-zA-Z ]+)\|([^{}]+)}}", text)) is not None:
        text = text.replace(r.group(0), "")

        if r.group(1) in ["short description"]:
            result[r.group(1)] = r.group(2)

    result["text"] = text
    return result

def extract_links(content: str) -> list[str]:
    links = []

    for link in re.findall(r"""\[\[([^\[\]]+)\]\]""", content):
        if "|" in link:
            link = link.split("|")[0]

        if "#" in link:
            link = link.split("#")[0]

        links.append(link)

    return links

def crawl(
    title: str,
    headless: bool = False,
) -> dict:
    print(f"Crawling {title}")
    link = generate_link(title)
    options = webdriver.ChromeOptions()

    if headless:
        options.add_argument("--headless")

    driver = webdriver.Chrome(options=options)
    driver.get(link)

    time.sleep(2)

    textarea = driver.find_elements(by=By.CSS_SELECTOR, value="div.wikiEditor-ui-text textarea")

    # protected pages
    if len(textarea) != 1:
        textarea = driver.find_elements(by=By.CSS_SELECTOR, value="div#mw-content-text textarea")
        assert len(textarea) == 1

    result = {
        'title': title,
        'text': textarea[0].text,
    }

    link = generate_synonym_link(title)
    driver.get(link)

    time.sleep(2)

    synonyms = set()

    for li in driver.find_elements(by=By.CSS_SELECTOR, value="ul#mw-whatlinkshere-list li a.mw-redirect"):
        if li.text not in ["edit", "links"]:
            synonyms.add(li.text)

    result['synonyms'] = list(synonyms)
    result['links'] = extract_links(result['text'])
    cleaned_text = clean_text(result['text'])

    for key, value in cleaned_text.items():
        result[key] = value

    return result

def save_result_to(
    result: dict,  # one that's returned from `search`
    path: str,
):
    import json

    result['visited'] = list(result['visited'])

    with open(path, "w") as f:
        f.write(json.dumps(result, ensure_ascii=False, indent=2))

    print(f"Saved to {path}")
    result['visited'] = set(result['visited'])

def try_load_result_from(
    path: str
) -> Optional[dict]:
    import json

    try:
        with open(path, "r") as f:
            result = json.load(f)
            result['visited'] = set(result['visited'])

        return result

    except FileNotFoundError:
        return None

def result_to_dir(
    result: dict,
    dir: str,
):
    # TODO: some titles include "/"
    for title, doc in result['docs'].items():
        with open(f"{dir}/{title}.txt", "w") as f:
            f.write(doc['text'])

def search(
    resume_from: Optional[dict],  # return value of bfs, if exists
    start: Optional[str],  # keyword to start crawling
    max_crawl: int = 500,  # set it to -1 to crawl infinitely
    headless: bool = True,

    # save_every: int, save_at: str,
    save_option: Optional[dict] = None,
) -> dict:
    result = resume_from
    run_count = 0

    if result is None:
        result = {
            'visited': set(),  # it includes synonyms
            'pending': [start],
            'docs': {},
        }

    while result['pending']:
        title = result['pending'].pop(0)

        if title in result['visited']:
            continue

        try:
            doc = crawl(title, headless=headless)
            run_count += 1
            result['docs'][title] = doc
            result['visited'].add(title)

            for synonym in doc['synonyms']:
                result['visited'].add(synonym)

            for link in doc['links']:
                result['pending'].append(link)

            if save_option is not None:
                if run_count % save_option['save_every'] == 0:
                    save_result_to(result, save_option['save_at'])

            max_crawl -= 1

            if max_crawl == 0:
                break

        except Exception as e:
            print(e)

    return result

if __name__ == "__main__":
    from tests import goto_root
    goto_root()
    command = sys.argv[1]

    if command == "haskell":
        start, json_at, sample_at = "Haskell", "sample/wiki-haskell/index.json", "sample/wiki-haskell"

    while True:
        result = search(
            resume_from=try_load_result_from(json_at),
            start=start,
            max_crawl=12,
            headless=True,
            save_option={
                'save_every': 3,
                'save_at': json_at,
            },
        )
        result_to_dir(result, sample_at)
