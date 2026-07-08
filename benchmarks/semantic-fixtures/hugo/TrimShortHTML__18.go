# ID: helpers/content.go:167
package helpers

import (
	"bytes"

	"github.com/gohugoio/hugo/media"
)

// unwrapSingleParagraph removes the wrapping paragraph tags when the input is a
// single paragraph after whitespace trimming.
func unwrapSingleParagraph(c *ContentSpec, input []byte, markup string) []byte {
	openingTag := []byte("<p>")
	closingTag := []byte("</p>")
	if markup == media.DefaultContentTypes.AsciiDoc.SubType {
		openingTag = []byte("<div class=\"paragraph\">\n<p>")
		closingTag = []byte("</p>\n</div>")
	}

	if bytes.Count(input, openingTag) != 1 {
		return input
	}

	input = bytes.TrimSpace(input)
	if bytes.HasPrefix(input, openingTag) && bytes.HasSuffix(input, closingTag) {
		input = bytes.TrimPrefix(input, openingTag)
		input = bytes.TrimSuffix(input, closingTag)
		input = bytes.TrimSpace(input)
	}
	return input
}
