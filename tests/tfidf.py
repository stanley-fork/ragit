from utils import cargo_run, goto_root, mk_and_cd_tmp_dir, write_string

lorem_ipsum1 = 'Suspendisse scelerisque accumsan gravida. Etiam nec viverra tortor. Praesent neque magna, fringilla id volutpat id, iaculis vitae risus. Aliquam vitae massa id diam ornare malesuada. Suspendisse accumsan erat non lacus placerat euismod. Suspendisse rutrum condimentum nibh, vitae fringilla lorem vulputate at. Vivamus pulvinar nisl eros, mattis suscipit erat consectetur at. Duis condimentum suscipit venenatis. Sed eu velit gravida, efficitur nisl ut, luctus lectus. Aliquam commodo commodo. Fusce blandit lobortis urna sit amet scelerisque. Fusce ut leo lorem. Vestibulum interdum euismod egestas. Maecenas sed sem metus.'
lorem_ipsum2 = 'In eget sem nisl. Nam convallis nunc leo, at venenatis turpis maximus a. Proin id nisi in arcu elementum ultrices. Duis aliquam nisi odio, ut gravida mi volutpat non. Pellentesque tincidunt sollicitudin tellus nec suscipit. Pellentesque non odio porttitor, eleifend erat eget, tempor erat. Aenean ut metus gravida, accumsan nibh vel, ultricies est. Duis nec mi vel purus laoreet elementum. Fusce convallis imperdiet diam, vitae ullamcorper enim ornare et.'

def tfidf():
    goto_root()
    mk_and_cd_tmp_dir()
    write_string("lorem_ipsum1.txt", lorem_ipsum1)
    write_string("lorem_ipsum2.txt", lorem_ipsum2)
    cargo_run(["init"])
    cargo_run(["config", "--set", "model", "dummy"])
    cargo_run(["check"])

    # step 1: tfidf without any chunk
    cargo_run(["tfidf", "hello, world"])

    # step 2: tfidf with only 1 chunk
    cargo_run(["add", "lorem_ipsum1.txt"])
    cargo_run(["build"])
    assert "lorem_ipsum1.txt" not in cargo_run(["tfidf", "hello, world"], stdout=True)
    assert "lorem_ipsum1.txt" in cargo_run(["tfidf", "Praesent neque magna"], stdout=True)

    # step 3: tfidf with multiple chunks
    cargo_run(["add", "lorem_ipsum2.txt"])
    cargo_run(["build"])
    assert "lorem_ipsum1.txt" in cargo_run(["tfidf", "Praesent neque magna"], stdout=True)
    assert "lorem_ipsum2.txt" not in cargo_run(["tfidf", "Praesent neque magna"], stdout=True)
    assert "lorem_ipsum1.txt" not in cargo_run(["tfidf", "Pellentesque tincidunt"], stdout=True)
    assert "lorem_ipsum2.txt" in cargo_run(["tfidf", "Pellentesque tincidunt"], stdout=True)

    # step 4: tfidf on cjk
    write_string("cjk.txt", "나는 비빔인간입니다.")
    cargo_run(["add", "cjk.txt"])
    cargo_run(["build"])
    cargo_run(["check"])
    assert "cjk.txt" in cargo_run(["tfidf", "비빔"], stdout=True)
    assert "lorem_ipsum1.txt" not in cargo_run(["tfidf", "비빔"], stdout=True)
    assert "cjk.txt" in cargo_run(["tfidf", "비빔인간"], stdout=True)
    assert "lorem_ipsum1.txt" not in cargo_run(["tfidf", "비빔인간"], stdout=True)
    assert "cjk.txt" not in cargo_run(["tfidf", "Pellentesque tincidunt"], stdout=True)
