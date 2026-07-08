# ID: src/Illuminate/Routing/RouteUrlGenerator.php:334
<?php
public static function substituteRouteWildcards($generator, $path, array &$parameters)
{
    // First resolve the named wildcards such as {user}
    $path = $generator->replaceNamedParameters($path, $parameters);

    // Then consume positional parameters for whatever wildcards remain
    $path = preg_replace_callback('/\{.*?\}/', function ($match) use (&$parameters) {
        // Re-index so only the numeric keys are reset
        $parameters = array_merge($parameters);

        return (! isset($parameters[0]) && ! str_ends_with($match[0], '?}'))
            ? $match[0]
            : Arr::pull($parameters, 0);
    }, $path);

    // Drop any leftover optional wildcards and tidy the surrounding slashes
    return trim(preg_replace('/\{.*?\?\}/', '', $path), '/');
}
