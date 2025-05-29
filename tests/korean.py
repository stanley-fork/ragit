import shutil
from utils import (
    cargo_run,
    goto_root,
    mk_and_cd_tmp_dir,
    write_string,
)

def korean():
    goto_root()
    mk_and_cd_tmp_dir()

    for i in range(2):
        features = [] if i == 0 else ["korean"]

        cargo_run(["init"])
        cargo_run(["config", "--set", "model", "dummy"])
        write_string("sample1.txt", "AI가 개발자를 대체하게 될까요?")
        write_string("sample2.txt", "나는 비빔인간입니다.")
        cargo_run(["add", "sample1.txt", "sample2.txt"])
        cargo_run(["build"], features=features)
        cargo_run(["check"])

        if i == 0:
            assert "sample1.txt" not in cargo_run(["tfidf", "개발자"], stdout=True)
            assert "sample2.txt" not in cargo_run(["tfidf", "개발자"], stdout=True)
            assert "sample1.txt" not in cargo_run(["tfidf", "비빔인간"], stdout=True)
            assert "sample2.txt" not in cargo_run(["tfidf", "비빔인간"], stdout=True)

        else:
            assert "sample1.txt" in cargo_run(["tfidf", "개발자"], stdout=True)
            assert "sample2.txt" not in cargo_run(["tfidf", "개발자"], stdout=True)
            assert "sample1.txt" not in cargo_run(["tfidf", "비빔인간"], stdout=True)
            assert "sample2.txt" in cargo_run(["tfidf", "비빔인간"], stdout=True)

        shutil.rmtree(".ragit")
