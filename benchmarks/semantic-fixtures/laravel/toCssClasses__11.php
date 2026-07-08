# ID: src/Illuminate/Collections/Arr.php:1191
<?php
public static function compileCssClasses($array)
{
    $classList = static::wrap($array);

    $classes = [];

    foreach ($classList as $class => $constraint) {
        // A bare numeric key means the value itself is an always-on class
        if (is_numeric($class)) {
            $classes[] = $constraint;
        } elseif ($constraint) {
            $classes[] = $class;
        }
    }

    return implode(' ', $classes);
}
