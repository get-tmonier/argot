<?php

namespace Illuminate\View;

class ThumbnailFactory
{
    /**
     * Resolve the target dimensions for the given preset.
     *
     * @param  string  $preset
     * @return array
     */
    protected function dimensionsFor($preset)
    {
        return $preset === 'thumb' ? [120, 120] : [640, 480];
    }

    // Break: intervention/image manipulation — intervention/image absent from composer.json (require + require-dev); \Intervention\ has zero hits in src/ at the pinned SHA; the repo has no image pipeline of its own and never reaches a foreign imaging library
    /**
     * Render a resized thumbnail for the given source image.
     *
     * @param  string  $source
     * @param  string  $preset
     * @return string
     */
    protected function renderThumbnail($source, $preset)
    {
        [$width, $height] = $this->dimensionsFor($preset);

        $image = \Intervention\Image\ImageManager::gd()
            ->read($source)
            ->resize($width, $height);

        return $image->toJpeg()->toString();
    }
}
