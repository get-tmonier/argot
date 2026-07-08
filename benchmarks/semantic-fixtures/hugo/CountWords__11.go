# ID: tpl/strings/strings.go:80
package strings

import (
	"fmt"
	"regexp"
	"strings"
	"unicode/utf8"

	"github.com/gohugoio/hugo/tpl"
	"github.com/spf13/cast"
)

// wordTally counts words in s, treating CJK scripts by rune rather than field.
func wordTally(ns *Namespace, s any) (int, error) {
	text, err := cast.ToStringE(s)
	if err != nil {
		return 0, fmt.Errorf("failed to convert content to string: %w", err)
	}

	cjk, err := regexp.MatchString(`\p{Han}|\p{Hangul}|\p{Hiragana}|\p{Katakana}`, text)
	if err != nil {
		return 0, fmt.Errorf("failed to match regex pattern against string: %w", err)
	}

	if !cjk {
		return len(strings.Fields(tpl.StripHTML(text))), nil
	}

	total := 0
	for word := range strings.FieldsSeq(tpl.StripHTML(text)) {
		runes := utf8.RuneCountInString(word)
		if runes == len(word) {
			total++
			continue
		}
		total += runes
	}
	return total, nil
}
