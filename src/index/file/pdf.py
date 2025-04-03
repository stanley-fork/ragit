import os
import pygame
import pypdfium2 as pdfium
import shutil
import sys
from typing import Optional

def count_pages(pdf_file: str) -> int:
    p = pdfium.PdfDocument(pdf_file)
    return len(p)

def convert_page_to_image(input: str, output: str, page: int, scale: int = 1):
    p = pdfium.PdfDocument(input)
    page = p.get_page(page)
    page.render(scale=scale).to_pil().save(output)

# chunk: None | "vert" | "horiz"
def convert_file(input: str, chunk: Optional[str] = None):
    pygame.init()
    pages_dir = f"{input}-pages"

    if os.path.exists(pages_dir):
        shutil.rmtree(pages_dir)
        print(f"Deleted {pages_dir}")

    os.mkdir(pages_dir)

    for page_no in range(count_pages(input)):
        image_path = os.path.join(pages_dir, f"page-{page_no + 1:04}.png")
        convert_page_to_image(input, image_path, page_no, scale = 4)
        image = pygame.image.load(image_path)
        w, h = image.get_size()

        if chunk in ["vert", "horiz"]:
            if chunk == "vert":
                for i in range(3):
                    surface = pygame.surface.Surface((w, h // 2))
                    surface.blit(image, (0, 0), (0, h * i // 4, w, h // 2))
                    image_sub_path = os.path.join(pages_dir, f"page-{page_no + 1:04}-chunk-{i:04}.png")
                    pygame.image.save(surface, image_sub_path)
                    print(f"Saved {image_sub_path}")

            else:
                for i in range(3):
                    surface = pygame.surface.Surface((w // 2, h))
                    surface.blit(image, (0, 0), (w * i // 4, 0, w // 2, h))
                    image_sub_path = os.path.join(pages_dir, f"page-{page_no + 1:04}-chunk-{i:04}.png")
                    pygame.image.save(surface, image_sub_path)
                    print(f"Saved {image_sub_path}")

            os.remove(image_path)

        else:
            print(f"Saved {image_path}")

help_message = """
Usage: python3 pdf.py <pdf-file> [--vert | --horiz]

By default, it does not split the pages into chunks.

Options:
    --vert: Split the pages vertically into 3 chunks.
    --horiz: Split the pages horizontally into 3 chunks.
"""

if __name__ == "__main__":
    pygame.init()

    if "--help" in sys.argv:
        print(help_message)
        sys.exit(0)

    pdf_file = sys.argv[1]
    chunk = "vert" if "--vert" in sys.argv else "horiz" if "--horiz" in sys.argv else None
    convert_file(pdf_file, chunk)
