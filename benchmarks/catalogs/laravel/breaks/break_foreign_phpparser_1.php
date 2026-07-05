<?php

namespace Illuminate\View;

use PhpParser\ParserFactory;

class TemplateInspector
{
    /**
     * Determine whether the given path is a PHP template.
     *
     * @param  string  $path
     * @return bool
     */
    protected function isPhpTemplate($path)
    {
        return str_ends_with($path, '.blade.php') || str_ends_with($path, '.php');
    }

    // Break: nikic/php-parser AST inspection — nikic/php-parser absent from composer.json (require + require-dev); PhpParser\ has zero hits in src/ at the pinned SHA. The factory is constructed by short name and driven through a receiver variable; the repo compiles templates through its own Blade compiler and never parses PHP into an AST.
    /**
     * Extract the top-level statements from the given template source.
     *
     * @param  string  $source
     * @return array
     */
    protected function extractStatements($source)
    {
        $factory = new ParserFactory();

        $parser = $factory->createForNewestSupportedVersion();

        $ast = $parser->parse($source);

        return $ast ?? [];
    }
}
