# Prequery preprocessor

[![GitHub Pages](https://img.shields.io/static/v1?logo=github&label=Pages&message=prequery&color=blue)](https://typst-community.github.io/prequery/)
[![GitHub repo](https://img.shields.io/static/v1?logo=github&label=Repo&message=prequery-preprocess&color=blue)](https://github.com/typst-community/prequery-preprocess)
[![GitHub tag](https://img.shields.io/github/tag/typst-community/prequery-preprocess?sort=semver&color=blue)](https://github.com/typst-community/prequery-preprocess/releases/)
[![License](https://img.shields.io/badge/License-MIT-blue)](https://github.com/typst-community/prequery-preprocess?tab=MIT-1-ov-file)
[![GitHub issues](https://img.shields.io/github/issues/typst-community/prequery-preprocess)](https://github.com/typst-community/prequery-preprocess/issues)

A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.
Typst compilations are sandboxed: it is not possible for Typst packages, or even just a Typst document itself, to access the "ouside world".
This sandboxing of Typst has good reasons.
Yet, it is often convenient to trade a bit of security for convenience by weakening it.
The `prequery` preprocessor is a CLI tool that works outside the Typst sandbox to do tasks such as download resources.
(In fact, this is the only feature currently implemented in the preprocessor.)

The full Prequery documentation, including the preprocessor, is located at https://typst-community.github.io/prequery/.
