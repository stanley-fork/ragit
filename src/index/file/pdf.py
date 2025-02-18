import os
import pygame
import pypdfium2 as pdfium
import sys

def count_pages(pdf_file: str) -> int:
    p = pdfium.PdfDocument(pdf_file)
    return len(p)

def convert_page_to_image(input: str, output: str, page: int, scale: int = 1):
    p = pdfium.PdfDocument(input)
    page = p.get_page(page)
    page.render(scale=scale).to_pil().save(output)

def convert_file(input: str):
    pygame.init()
    pages_dir = f"{input}-pages"
    os.mkdir(pages_dir)

    for page_no in range(count_pages(input)):
        image_path = os.path.join(pages_dir, f"page-{page_no:04}.png")
        convert_page_to_image(input, image_path, page_no, scale = 4)
        image = pygame.image.load(image_path)
        w, h = image.get_size()

        for i in range(3):
            surface = pygame.surface.Surface((w, h // 2))
            surface.blit(image, (0, 0), (0, h * i // 4, w, h // 2))
            image_sub_path = os.path.join(pages_dir, f"page-{page_no:04}-chunk-{i:04}.png")
            pygame.image.save(surface, image_sub_path)
            print(f"Saved {image_sub_path}")

        os.remove(image_path)

if __name__ == "__main__":
    pygame.init()

    pdf_file = sys.argv[1]
    convert_file(pdf_file)
