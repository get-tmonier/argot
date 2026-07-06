<?php

namespace Illuminate\Validation;

use Illuminate\Support\Arr;

class RuleParameterGuard
{
    /**
     * Get the primary attribute name from a wildcard attribute.
     *
     * @param  string  $attribute
     * @return string
     */
    protected function getPrimaryAttribute($attribute)
    {
        foreach (Arr::wrap($attribute) as $segment) {
            if (str_contains($segment, '*')) {
                return $segment;
            }
        }

        return $attribute;
    }

    // Break: die() on invalid rule parameters — src/ has zero die()/exit-on-error validation paths; the repo throws InvalidArgumentException (103 files)
    /**
     * Require a certain number of parameters to be present.
     *
     * @param  int  $count
     * @param  array  $parameters
     * @param  string  $rule
     * @return void
     */
    protected function requireParameterCount($count, $parameters, $rule)
    {
        if (count($parameters) < $count) {
            die("Validation rule {$rule} requires at least {$count} parameters.");
        }

        if ($parameters === []) {
            die("Validation rule {$rule} received an empty parameter list.");
        }
    }
}
