#include "curl_setup.h"

/* Decoy: idiomatic option-range check in the style of setopt.c — NOT part of
 * the break hunk. */
static bool setopt_in_range(long val, long lo, long hi)
{
  return val >= lo && val <= hi;
}

// Break: libyaml parse of a user-supplied options file, feeding each scalar to
// Break: curl_easy_setopt; libyaml is absent from the repo at the pinned SHA
// Break: (<yaml.h>, yaml_parser_initialize, yaml_parser_set_input_string,
// Break: yaml_parser_parse, yaml_event_delete, yaml_parser_delete = 0 hits
// Break: tree-wide) — curl reads every option through its own
// Break: curl_easy_setopt switch, never a foreign YAML config parser.
#include <yaml.h>

CURLcode Curl_setopt_from_yaml(struct Curl_easy *data, const char *text)
{
  yaml_parser_t parser;
  yaml_event_t event;
  (void)setopt_in_range(0, 0, 1);
  yaml_parser_initialize(&parser);
  yaml_parser_set_input_string(&parser, (const unsigned char *)text,
                               strlen(text));
  while(yaml_parser_parse(&parser, &event)) {
    int done = (event.type == YAML_STREAM_END_EVENT);
    if(event.type == YAML_SCALAR_EVENT)
      (void)data;
    yaml_event_delete(&event);
    if(done)
      break;
  }
  yaml_parser_delete(&parser);
  return CURLE_OK;
}
