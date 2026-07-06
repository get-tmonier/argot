<?php

namespace Illuminate\Support;

class StringFormatter
{
    /**
     * Convert the given string to proper case.
     *
     * @param  string  $value
     * @return string
     */
    public static function title($value)
    {
        return mb_convert_case($value, MB_CASE_TITLE, 'UTF-8');
    }

    // Break: snake_case public methods — zero snake_case public methods in src/ (camelCase public methods across 667 files); method morphology is foreign to the repo
    /**
     * Pad both sides of the given string.
     *
     * @param  string  $value
     * @param  int  $length
     * @param  string  $pad
     * @return string
     */
    public static function pad_both_sides($value, $length, $pad = ' ')
    {
        return str_pad($value, $length, $pad, STR_PAD_BOTH);
    }

    /**
     * Convert the given string to studly caps with a cache.
     *
     * @param  string  $value
     * @return string
     */
    public static function make_studly_cached($value)
    {
        static $studly_cache = [];

        $cache_key = $value;

        if (isset($studly_cache[$cache_key])) {
            return $studly_cache[$cache_key];
        }

        $word_list = explode(' ', str_replace(['-', '_'], ' ', $value));

        return $studly_cache[$cache_key] = implode('', array_map('ucfirst', $word_list));
    }
}
