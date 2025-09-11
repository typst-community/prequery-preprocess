# Prequery preprocessor

A tool for processing [prequery](https://typst.app/universe/package/prequery) data in Typst documents.
Typst compilations are sandboxed: it is not possible for Typst packages, or even just a Typst document itself, to access the "ouside world".
This sandboxing of Typst has good reasons.
Yet, it is often convenient to trade a bit of security for convenience by weakening it.
The `prequery` preprocessor is a CLI tool that works outside the Typst sandbox to do tasks such as download resources.
(In fact, this is the only feature currently implemented in the preprocessor.)

The full Prequery documentation, including the preprocessor, is located at https://typst-community.github.io/prequery/.
