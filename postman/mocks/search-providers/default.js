const http = require('http');
const hashes = ['a','b','c','d'].map(c => c.repeat(40));
http.createServer((req,res) => {
  const u = new URL(req.url, 'http://localhost');
  const query = u.searchParams.get('q') || u.searchParams.get('search') || u.searchParams.get('query_term') || u.searchParams.get('s') || '';
  res.setHeader('Content-Type', 'application/json');
  const send = body => res.end(JSON.stringify(body));
  if (query === 'outage') {res.statusCode=503; return send({error:'Fixture provider unavailable'});}
  if (query === 'Ubuntu-partial' && u.pathname === '/piratebay/q.php') {res.statusCode=503;return send({error:'Fixture partial outage'});}
  if (query === 'malformed') {res.setHeader('Content-Type','text/html');return res.end('<html>upstream failure</html>');}
  if (u.pathname === '/piratebay/q.php') return send([{id:'42',name:'Ubuntu Fixture',info_hash:hashes[0],size:'1048576',seeders:'50',leechers:'2',category:'300'}]);
  if (u.pathname === '/fitgirl/wp-json/wp/v2/posts') return send([{id:7,link:'https://example.org/fixture',title:{rendered:'Ubuntu Fixture – FitGirl Repack'},content:{rendered:`<p>Repack Size: 1.5 GB</p><a href="magnet:?xt=urn:btih:${hashes[1]}&amp;dn=Ubuntu">Magnet</a><p>Fixture &amp; description</p>`}}]);
  if (u.pathname === '/fitgirl/feed/') {res.setHeader('Content-Type','application/xml');return res.end('<rss><channel></channel></rss>');}
  if (u.pathname === '/yts/api/v2/list_movies.json') return send({status:'ok',data:{movies:[{id:1,title:'Ubuntu Fixture',year:2026,slug:'ubuntu-fixture',summary:'Fixture summary',torrents:[{hash:hashes[2],quality:'1080p',type:'web',seeds:20,peers:3,size_bytes:2048}]}]}});
  if (u.pathname === '/tvmaze/search/shows') return send([{show:{id:1,name:'Ubuntu Fixture',externals:{imdb:'tt1234567'}}}]);
  if (u.pathname === '/eztv/api/get-torrents') return send({torrents:[{hash:hashes[3],filename:'Ubuntu Fixture S01E01',size_bytes:'4096',seeds:'30',peers:'4',episode_url:'https://example.org/episode'}]});
  res.statusCode=501;send({error:'Fixture endpoint not defined'});
}).listen(process.env.PORT);
