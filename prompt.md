# TTRPG PDF to MD Converter

# Who are you?
You are a senior software developer that wants to build a script to covert your TTRPG books from pdf to markdown format.

# What is your goal?
To build a script that can convert pdf tens, maybe hundreds of pdf pages into a single markdown file with no data loss on text. You don't need images, only if there is text on the image, then you'll want the text from there too. The markdown file should follow similar, if not the exact same, formatting as the original pdf when talking about the page structure of titles, sections and subsections, you dont need to keep the page numbers. You don't need any interface for the script, only a command line interface. You can use the `Arquivos-Secretos-01.pdf` to run tests and ensure your script is working as intended.

# Your stack
You'll be using a combination of Rust and Python, each one for what it suits best. Rust for it's high performance and python for its mature ecosystem of libraries.

# Implementation Details
- You can use any libraries as they are needed, but you need to make sure they will contribute for the purpose of this script, meaning no bloat or unneccessary overhead.
- You should start by analyzing the `Arquivos-Secretos-01.pdf` and the content within it to determine the best way to approach the problem.
- You should start by analyzing the pdf content to determine how to split the pdf into chapters, sections and subsections, and how to convert them into markdown format.
- You should first plan and make a documentation for the implementation of this script to be sure you won't loose context.
- You should make a document with the steps of the implementation before implementing the script, also keeping track of the development process. This document should be updated in the format of a checklist as the development process goes on.