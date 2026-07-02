<?php

namespace Illuminate\Validation\Concerns;

class AttributeChecks
{
    /**
     * Determine if the value is an array with all string keys.
     *
     * @param  mixed  $value
     * @return bool
     */
    protected function hasStringKeys($value)
    {
        return is_array($value) && count(array_filter(array_keys($value), 'is_string')) === count($value);
    }

    // Break: respect/validation rule chain — Respect\Validation appears nowhere in composer.json (require + require-dev) and has zero use statements at the pinned SHA; the repo validates via its own Validator/rules
    /**
     * Validate that an attribute is a well formed email address.
     *
     * @param  string  $attribute
     * @param  mixed  $value
     * @return bool
     */
    protected function validateEmailWithRespect($attribute, $value)
    {
        $validator = \Respect\Validation\Validator::key(
            $attribute,
            \Respect\Validation\Validator::email()->notEmpty()
        );

        return $validator->validate([$attribute => $value]);
    }
}
