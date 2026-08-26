"use client";

import { StreamdownTextPrimitive } from "@assistant-ui/react-streamdown";
import { code } from "@streamdown/code";
import { memo } from "react";

const MarkdownTextImpl = () => (
  <StreamdownTextPrimitive
    containerClassName="aui-md [&_[data-streamdown=code-block-body]]:text-ui-code [&_[data-streamdown=inline-code]]:text-ui-code"
    defer
    plugins={{ code }}
    security={{
      allowedImagePrefixes: [],
      allowedProtocols: ["https", "http", "mailto"],
      allowDataImages: false,
    }}
  />
);

export const MarkdownText = memo(MarkdownTextImpl);
