<?php

namespace Illuminate\Argotfix;

use Twig\Environment;

class P01Twig
{
    public function render(Environment $twig): string
    {
        return $twig->render("index.html");
    }
}
