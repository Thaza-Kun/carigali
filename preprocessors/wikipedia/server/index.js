// Server

const wtf = require('wtf_wikipedia')
wtf.extend(require('wtf-plugin-markdown'))
const parseMD = (...args) => import('parse-md').then(({ default: parseMD }) => parseMD(...args))

const yaml = require('js-yaml');

// We parse `content -> ::text() -> ::markdown()` as oppose to directly `content -> ::markdown()` 
// to remove inline styling (links, emph, etc.) but want to keep block-level style (heading, list, etc.)
async function wikitext_to_md(data) {
    let { metadata, content } = await parseMD(data);
    let mdata = {
        ...metadata,
        revision: [{
            ...metadata.revision[0],
            ["comment"]: wtf(wtf(metadata.revision[0].comment).text()).markdown()
        }
        ]
    }
    let parsed_content = wtf(wtf(content).text()).markdown()
    return `---\n${yaml.dump(mdata)}---\n${parsed_content}`
}

console.log("Server running at localhost:3010")
Bun.serve({
    hostname: "localhost",
    port: 3010,
    async fetch(req) {
        const url = new URL(req.url);
        if (url.pathname === "/") return new Response("", { status: 200 });
        if (url.pathname === "/wikitext/") return Response.redirect('/wikitext/');
        if (url.pathname === "/wikitext") return await req.json().then(
            async (a) => await wikitext_to_md(a['content']).then((res) => new Response(res, { status: 200 }), (err) => new Response('Malformed json payload', { status: 400 })),
            (err) => {
                console.log(err)
                console.log(req.body.text)
                return new Response("Malformed json payload", { status: 400 })
            });
        return new Response(`404! ${url.pathname}`, { status: 404 });
    },
});