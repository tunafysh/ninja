"use client"

import { useRef, useState, useEffect } from "react"
import dynamic from "next/dynamic"
import Editor, { type Monaco, loader } from "@monaco-editor/react"
import { Sun, Moon } from "lucide-react"
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "@/components/ui/resizable"

// Dynamically import Terminal component with no SSR
const TerminalComponent = dynamic(() => import("./terminal-component"), {
  ssr: false,
  loading: () => (
    <div className="flex items-center justify-center h-full">
      <div>Loading terminal...</div>
    </div>
  ),
})

// Define the themes before Monaco loads
loader.init().then((monaco) => {
  // One Dark theme (dark mode)
  monaco.editor.defineTheme("oneDark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "5c6370", fontStyle: "italic" },
      { token: "keyword", foreground: "c678dd" },
      { token: "string", foreground: "98c379" },
      { token: "number", foreground: "d19a66" },
      { token: "attribute.name", foreground: "e5c07b" },
      { token: "attribute.value", foreground: "98c379" },
      { token: "tag.bracket", foreground: "abb2bf" },
      { token: "tag.directive", foreground: "61afef" },
      { token: "tag.parameter", foreground: "d19a66" },
      { token: "variable", foreground: "e06c75" },
      { token: "constant.numeric", foreground: "d19a66" },
      { token: "operator", foreground: "56b6c2" },
      { token: "delimiter", foreground: "abb2bf" },
    ],
    colors: {
      "editor.background": "#282c34",
      "editor.lineHighlightBackground": "#2c313c",
      "editor.foreground": "#abb2bf",
      "editorCursor.foreground": "#528bff",
      "editor.selectionBackground": "#3E4451",
      "editorWhitespace.foreground": "#3B4048",
    },
  })

  // GitHub Light theme (light mode)
  monaco.editor.defineTheme("githubLight", {
    base: "vs",
    inherit: true,
    rules: [
      { token: "comment", foreground: "6a737d", fontStyle: "italic" },
      { token: "keyword", foreground: "d73a49" },
      { token: "string", foreground: "032f62" },
      { token: "number", foreground: "005cc5" },
      { token: "attribute.name", foreground: "6f42c1" },
      { token: "attribute.value", foreground: "032f62" },
      { token: "tag.bracket", foreground: "24292e" },
      { token: "tag.directive", foreground: "22863a" },
      { token: "tag.parameter", foreground: "e36209" },
      { token: "variable", foreground: "e36209" },
      { token: "constant.numeric", foreground: "005cc5" },
      { token: "operator", foreground: "d73a49" },
      { token: "delimiter", foreground: "24292e" },
    ],
    colors: {
      "editor.background": "#ffffff",
      "editor.lineHighlightBackground": "#f6f8fa",
      "editor.foreground": "#24292e",
      "editorCursor.foreground": "#044289",
      "editor.selectionBackground": "#c8e1ff",
      "editorWhitespace.foreground": "#d1d5da",
    },
  })
})

// Register the Apache language
function registerApacheLanguage(monaco: Monaco) {
  // Check if the language is already registered
  if (monaco.languages.getLanguages().some((lang) => lang.id === "apache")) {
    return // Already registered
  }

  // Register a new language
  monaco.languages.register({ id: "apache" })

  // Register a tokens provider for the language
  monaco.languages.setMonarchTokensProvider("apache", {
    // Set defaultToken to invalid to see what you do not tokenize yet
    defaultToken: "invalid",

    // Apache directives are case-insensitive
    ignoreCase: true,

    // The main tokenizer for our languages
    tokenizer: {
      root: [
        // Comments
        [/#.*$/, "comment"],

        // Opening tags with parameters - match in specific order
        [/<(?=\/?[A-Za-z])/, "tag.bracket"], // Opening bracket
        [/\/(?=[A-Za-z]+>)/, "tag.bracket"], // Forward slash in closing tags
        [/[A-Za-z]+(?=\s|>|\/>)/, "tag.directive"], // Directive name
        [/\s+[^>]+(?=>)/, "tag.parameter"], // Parameters
        [/>/, "tag.bracket"], // Closing bracket

        // Common Apache directives
        [
          /\b(AccessFileName|AddDefaultCharset|AddDescription|AddEncoding|AddHandler|AddIcon|AddIconByEncoding|AddIconByType|AddLanguage|AddModule|AddModuleInfo|AddOutputFilter|AddOutputFilterByType|AddType|Alias|AliasMatch|Allow|AllowCONNECT|AllowEncodedSlashes|AllowMethods|AllowOverride|Anonymous|Anonymous_Authoritative|Anonymous_LogEmail|Anonymous_MustGiveEmail|Anonymous_NoUserID|Anonymous_VerifyEmail|AuthAuthoritative|AuthDBAuthoritative|AuthDBGroupFile|AuthDBMAuthoritative|AuthDBMGroupFile|AuthDBMType|AuthDBMUserFile|AuthDBUserFile|AuthDigestAlgorithm|AuthDigestDomain|AuthDigestFile|AuthDigestGroupFile|AuthDigestNcCheck|AuthDigestNonceFormat|AuthDigestNonceLifetime|AuthDigestQop|AuthDigestShmemSize|AuthGroupFile|AuthLDAPAuthoritative|AuthLDAPBindDN|AuthLDAPBindPassword|AuthLDAPCharsetConfig|AuthLDAPCompareDNOnServer|AuthLDAPDereferenceAliases|AuthLDAPEnabled|AuthLDAPFrontPageHack|AuthLDAPGroupAttribute|AuthLDAPGroupAttributeIsDN|AuthLDAPRemoteUserIsDN|AuthLDAPUrl|AuthName|AuthType|AuthUserFile|BrowserMatch|BrowserMatchNoCase|BS2000Account|BufferedLogs|CacheDefaultExpire|CacheDirLength|CacheDirLevels|CacheDisable|CacheEnable|CacheExpiryCheck|CacheFile|CacheForceCompletion|CacheGcClean|CacheGcDaily|CacheGcInterval|CacheGcMemUsage|CacheGcUnused|CacheIgnoreCacheControl|CacheIgnoreHeaders|CacheIgnoreNoLastMod|CacheLastModifiedFactor|CacheMaxExpire|CacheMaxFileSize|CacheMinFileSize|CacheNegotiatedDocs|CacheRoot|CacheSize|CacheTimeMargin|CGIMapExtension|CharsetDefault|CharsetOptions|CharsetSourceEnc|CheckSpelling|ChildPerUserID|ContentDigest|CookieDomain|CookieExpires|CookieLog|CookieName|CookieStyle|CookieTracking|CoreDumpDirectory|CustomLog|Dav|DavDepthInfinity|DavLockDB|DavMinTimeout|DefaultIcon|DefaultLanguage|DefaultType|DeflateBufferSize|DeflateCompressionLevel|DeflateFilterNote|DeflateMemLevel|DeflateWindowSize|Deny|DirectoryIndex|DirectorySlash|DocumentRoot|DumpIOInput|DumpIOOutput|EnableExceptionHook|EnableMMAP|EnableSendfile|ErrorDocument|ErrorLog|Example|ExpiresActive|ExpiresByType|ExpiresDefault|ExtendedStatus|ExtFilterDefine|ExtFilterOptions|FileETag|ForceLanguagePriority|ForceType|ForensicLog|Group|Header|HeaderName|HostnameLookups|IdentityCheck|IdentityCheckTimeout|ImapBase|ImapDefault|ImapMenu|Include|IndexIgnore|IndexOptions|IndexOrderDefault|ISAPIAppendLogToErrors|ISAPIAppendLogToQuery|ISAPICacheFile|ISAPIFakeAsync|ISAPILogNotSupported|ISAPIReadAheadBuffer|KeepAlive|KeepAliveTimeout|LanguagePriority|LDAPCacheEntries|LDAPCacheTTL|LDAPConnectionTimeout|LDAPOpCacheEntries|LDAPOpCacheTTL|LDAPSharedCacheFile|LDAPSharedCacheSize|LDAPTrustedCA|LDAPTrustedCAType|LimitInternalRecursion|LimitRequestBody|LimitRequestFields|LimitRequestFieldsize|LimitRequestLine|LimitXMLRequestBody|Listen|ListenBacklog|LoadFile|LoadModule|LockFile|LogFormat|LogLevel|MaxClients|MaxKeepAliveRequests|MaxMemFree|MaxRequestsPerChild|MaxRequestsPerThread|MaxSpareServers|MaxSpareThreads|MaxThreads|MaxThreadsPerChild|MCacheMaxObjectCount|MCacheMaxObjectSize|MCacheMaxStreamingBuffer|MCacheMinObjectSize|MCacheRemovalAlgorithm|MCacheSize|MetaDir|MetaFiles|MetaSuffix|MimeMagicFile|MinSpareServers|MinSpareThreads|MMapFile|ModMimeUsePathInfo|MultiviewsMatch|NameVirtualHost|NoCache|NoProxy|NumServers|NWSSLTrustedCerts|NWSSLUpgradeable|Options|Order|PassEnv|PidFile|ProtocolEcho|ProxyBadHeader|ProxyBlock|ProxyDomain|ProxyErrorOverride|ProxyFtpDirCharset|ProxyIOBufferSize|ProxyMaxForwards|ProxyPass|ProxyPassReverse|ProxyPreserveHost|ProxyReceiveBufferSize|ProxyRemote|ProxyRemoteMatch|ProxyRequests|ProxyTimeout|ProxyVia|ReadmeName|Redirect|RedirectMatch|RedirectPermanent|RedirectTemp|RemoveCharset|RemoveEncoding|RemoveHandler|RemoveInputFilter|RemoveLanguage|RemoveOutputFilter|RemoveType|RequestHeader|Require|RewriteBase|RewriteCond|RewriteEngine|RewriteLock|RewriteLog|RewriteLogLevel|RewriteMap|RewriteOptions|RewriteRule|RLimitCPU|RLimitMEM|RLimitNPROC|Satisfy|ScoreBoardFile|Script|ScriptAlias|ScriptAliasMatch|ScriptInterpreterSource|ScriptLog|ScriptLogBuffer|ScriptLogLength|ScriptSock|SecureListen|SendBufferSize|ServerAdmin|ServerAlias|ServerLimit|ServerName|ServerPath|ServerRoot|ServerSignature|ServerTokens|SetEnv|SetEnvIf|SetEnvIfNoCase|SetHandler|SetInputFilter|SetOutputFilter|SSIEndTag|SSIErrorMsg|SSIStartTag|SSITimeFormat|SSIUndefinedEcho|SSLCACertificateFile|SSLCACertificatePath|SSLCARevocationFile|SSLCARevocationPath|SSLCertificateChainFile|SSLCertificateFile|SSLCertificateKeyFile|SSLCipherSuite|SSLEngine|SSLMutex|SSLOptions|SSLPassPhraseDialog|SSLProtocol|SSLProxyCACertificateFile|SSLProxyCACertificatePath|SSLProxyCARevocationFile|SSLProxyCARevocationPath|SSLProxyCipherSuite|SSLProxyEngine|SSLProxyMachineCertificateFile|SSLProxyMachineCertificatePath|SSLProxyProtocol|SSLProxyVerify|SSLProxyVerifyDepth|SSLRandomSeed|SSLRequire|SSLRequireSSL|SSLSessionCache|SSLSessionCacheTimeout|SSLVerifyClient|SSLVerifyDepth|StartServers|StartThreads|SuexecUserGroup|ThreadLimit|ThreadsPerChild|ThreadStackSize|TimeOut|TransferLog|TypesConfig|UnsetEnv|UseCanonicalName|User|UserDir|VirtualDocumentRoot|VirtualDocumentRootIP|VirtualScriptAlias|VirtualScriptAliasIP|Win32DisableAcceptEx|XBitHack)\b/,
          "keyword",
        ],

        // Variables
        [/\${[^}]*}/, "variable"],
        [/%\{[^}]*\}/, "variable"],

        // Strings
        [/"([^"\\]|\\.)*$/, "string.invalid"], // non-terminated string
        [/'([^'\\]|\\.)*$/, "string.invalid"], // non-terminated string
        [/"/, "string", "@string_double"],
        [/'/, "string", "@string_single"],

        // Numbers
        [/\b\d+\b/, "number"],

        // IP addresses
        [/\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b/, "constant.numeric"],

        // Operators
        [/[=!<>]/, "operator"],
      ],

      string_double: [
        [/[^\\"]+/, "string"],
        [/\\./, "string.escape"],
        [/"/, "string", "@pop"],
      ],

      string_single: [
        [/[^\\']+/, "string"],
        [/\\./, "string.escape"],
        [/'/, "string", "@pop"],
      ],
    },
  })
}

export default function ApacheEditor() {
  const editorRef = useRef(null)
  const [isThemeReady, setIsThemeReady] = useState(false)
  const [isDarkMode, setIsDarkMode] = useState(true)

  // Check if theme is ready
  useEffect(() => {
    loader.init().then(() => {
      setIsThemeReady(true)
    })
  }, [])

  const handleEditorDidMount = (editor: any, monaco: Monaco) => {
    editorRef.current = editor
    // Register Apache language
    registerApacheLanguage(monaco)

    // Force theme update
    monaco.editor.setTheme(isDarkMode ? "oneDark" : "githubLight")
  }

  const toggleTheme = () => {
    setIsDarkMode(!isDarkMode)
    if (editorRef.current) {
      const monaco = (window as any).monaco
      if (monaco) {
        monaco.editor.setTheme(isDarkMode ? "githubLight" : "oneDark")
      }
    }
  }

  const apacheConfig = `# Apache configuration example
<VirtualHost *:80>
    ServerAdmin webmaster@example.com
    ServerName example.com
    ServerAlias www.example.com
    DocumentRoot /var/www/html
    ErrorLog \${APACHE_LOG_DIR}/error.log
    CustomLog \${APACHE_LOG_DIR}/access.log combined
    
    <Directory /var/www/html>
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
    </Directory>
    
    # Setup for mod_rewrite
    RewriteEngine On
    RewriteCond %{HTTP_HOST} ^www\\.(.*)$ [NC]
    RewriteRule ^(.*)$ https://%1/$1 [R=301,L]
</VirtualHost>`

  if (!isThemeReady) {
    return (
      <div className="flex flex-col w-full h-full">
        <h2 className="text-xl font-bold mb-4">Apache Configuration Syntax Highlighter</h2>
        <div className="border rounded-md overflow-hidden" style={{ height: "500px" }}>
          <div className="flex items-center justify-center h-full bg-gray-100 dark:bg-gray-800">
            <div className="text-gray-500 dark:text-gray-400">Loading editor...</div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className={`flex flex-col w-full h-full ${isDarkMode ? "dark" : ""}`}>
      <div className="flex justify-between items-center mb-4">
        <h2 className="text-xl font-bold">Apache Configuration Syntax Highlighter</h2>
        <button
          onClick={toggleTheme}
          className={`p-2 rounded-full ${isDarkMode ? "bg-gray-700 text-yellow-300" : "bg-blue-100 text-blue-800"}`}
          aria-label={isDarkMode ? "Switch to light mode" : "Switch to dark mode"}
        >
          {isDarkMode ? <Sun size={20} /> : <Moon size={20} />}
        </button>
      </div>

      <div className="h-[calc(100vh-200px)] border rounded-md overflow-hidden">
        <ResizablePanelGroup direction="vertical">
          <ResizablePanel defaultSize={60} minSize={20}>
            <div className="h-full p-0">
              <Editor
                height="100%"
                defaultLanguage="apache"
                defaultValue={apacheConfig}
                theme={isDarkMode ? "oneDark" : "githubLight"}
                onMount={handleEditorDidMount}
                options={{
                  minimap: { enabled: true },
                  scrollBeyondLastLine: false,
                  fontSize: 14,
                  lineNumbers: "on",
                  renderLineHighlight: "all",
                  roundedSelection: false,
                  selectOnLineNumbers: true,
                  wordWrap: "on",
                  automaticLayout: true,
                }}
                loading={
                  <div
                    className={`flex items-center justify-center h-full ${isDarkMode ? "bg-gray-800 text-gray-400" : "bg-gray-100 text-gray-500"}`}
                  >
                    <div>Loading editor...</div>
                  </div>
                }
              />
            </div>
          </ResizablePanel>

          <ResizableHandle withHandle />

          <ResizablePanel defaultSize={40} minSize={10}>
            <div className={`h-full ${isDarkMode ? "bg-[#282c34]" : "bg-white"}`}>
              <TerminalComponent isDarkMode={isDarkMode} />
            </div>
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  )
}

