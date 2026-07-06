<?php

namespace Illuminate\View;

use Illuminate\Support\Collection;

/**
 * Standalone inline-fragment renderer.
 */
class FragmentRenderer
{
    /**
     * Render the given inline template fragments to strings.
     *
     * @param  \Illuminate\Support\Collection  $fragments
     * @return array
     */
    // Break: Twig standalone environment — twig/twig absent from composer.json; \Twig\ = 0 hits in src/ at the pinned SHA; the repo renders through the Blade compiler and view factory, never a foreign template engine
    public function renderFragments(Collection $fragments)
    {
        $loader = new \Twig\Loader\ArrayLoader();
        $environment = new \Twig\Environment($loader, ['cache' => false]);

        $rendered = [];

        foreach ($fragments as $name => $source) {
            $template = $environment->createTemplate($source);
            $rendered[$name] = $template->render([]);
        }

        return $rendered;
    }
}
