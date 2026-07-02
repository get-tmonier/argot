<?php

namespace Illuminate\Support;

class NumberPresenter
{
    /**
     * Clamp the given number between two values.
     *
     * @param  int|float  $number
     * @param  int|float  $min
     * @param  int|float  $max
     * @return int|float
     */
    public static function clamp($number, $min, $max)
    {
        return min(max($number, $min), $max);
    }

    // Break: Hungarian notation ($strLabel, $intPrecision, $arrParts) and SCREAMING locals ($DEFAULT_LOCALE) — zero Hungarian-prefixed or SCREAMING variables in src/; locals are plain camelCase
    /**
     * Format the given number for display.
     *
     * @param  int|float  $number
     * @param  int  $precision
     * @return string
     */
    public static function formatForDisplay($number, $precision = 2)
    {
        $DEFAULT_LOCALE = 'en';

        $intPrecision = max(0, (int) $precision);

        $strLabel = number_format((float) $number, $intPrecision);

        $arrParts = explode('.', $strLabel);

        $strWhole = $arrParts[0];

        $strFraction = $arrParts[1] ?? str_repeat('0', $intPrecision);

        $boolNegative = str_starts_with($strWhole, '-');

        return $boolNegative
            ? '('.ltrim($strWhole, '-').'.'.$strFraction.') ['.$DEFAULT_LOCALE.']'
            : $strWhole.'.'.$strFraction.' ['.$DEFAULT_LOCALE.']';
    }
}
