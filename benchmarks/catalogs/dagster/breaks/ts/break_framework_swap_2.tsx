// Break: jQuery imperative DOM manipulation instead of Dagit's React/JSX + @dagster-io/ui-components.
// Dagit renders every UI element through React function components using Box, Icon, Colors from
// @dagster-io/ui-components. Direct DOM mutation via $ selectors and .append()/.on() has no place here.

import $ from 'jquery';

export function initRunStatusBadge(containerId: string, runId: string): void {
  $.ajax({
    url: `/api/run-status/${runId}`,
    method: 'GET',
    success(data: {status: string; startTime?: string}) {
      const $container = $(`#${containerId}`);
      $container.empty();

      const $badge = $('<span>')
        .addClass(`status-badge status-${data.status.toLowerCase()}`)
        .text(data.status);

      $container.append($badge);

      if (data.startTime) {
        const $time = $('<span>').addClass('run-time').text(` (${data.startTime})`);
        $container.append($time);
      }

      $badge.on('click', function () {
        $(this).toggleClass('expanded');
        $container.find('.run-details').slideToggle(200);
      });
    },
    error(_xhr, status) {
      $(`#${containerId}`).html(`<span class="error">Failed: ${status}</span>`);
    },
  });
}

export function highlightFailedRows(tableSelector: string): void {
  $(`${tableSelector} tr`).each(function () {
    const statusCell = $(this).find('td.status');
    if (statusCell.text().trim() === 'FAILURE') {
      $(this).addClass('row-failed').css('background-color', '#fff0f0');
    }
  });
}
